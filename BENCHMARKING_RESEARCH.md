# Engineering a High-Fidelity Web Benchmarking Harness in Rust

> A Chromiumoxide Implementation Guide

## 1. Introduction: The Paradigm Shift in Web Performance Engineering

The discipline of web performance measurement has undergone a fundamental transformation in the last decade, shifting from a server-centric view of latency to a user-centric model of perception. In the early epochs of the web, performance was often synonymous with "Time to Last Byte" (TTLB) or the firing of the `window.onload` event. These metrics, while easily measurable, have become increasingly decoupled from the actual user experience in modern application architectures. With the rise of Single Page Applications (SPAs), aggressive hydration strategies, and complex client-side state management, a page can be technically "loaded" long before it is interactive, or conversely, functional while background processes continue to churn.

For the modern systems engineer, particularly one operating within the Rust ecosystem, the challenge is to construct a benchmarking harness that transcends these archaic metrics. The goal is not merely to time how long a network request takes, but to quantify the fluidity of the interaction—the tactile responsiveness of a button click, the visual stability of a layout during hydration, and the perceptual availability of primary content. This requires a toolchain that is capable of deep introspection into the browser's rendering pipeline, executing with the precision and safety that strictly typed systems provide.

This report outlines the comprehensive architectural and implementation strategy for building a State-of-the-Art (SOTA) benchmarking harness using Rust and the `chromiumoxide` library. By leveraging Rust's zero-cost abstractions and memory safety alongside the Chrome DevTools Protocol (CDP), we can achieve a level of granularity and concurrency that is difficult to sustain in traditional Node.js-based environments.[^1] The resulting system will not only integrate seamlessly with existing UI testing workflows but will also provide a scalable foundation for rigorous, statistical performance analysis of "Critical User Journeys" (CUJs).

### 1.1 The Role of Rust in Performance Infrastructure

The choice of Rust for this infrastructure is strategic. While the JavaScript ecosystem (Puppeteer, Playwright) is the native home of browser automation, it suffers from the inherent unpredictability of garbage collection and the single-threaded event loop of Node.js. In a benchmarking context, the observer effect is real: the tool measuring the performance must not introduce jitter into the measurement. Rust's predictable runtime characteristics ensure that the harness itself remains lightweight and deterministic. Furthermore, the `chromiumoxide` crate provides a robust, asynchronous API that maps the vast, loosely typed JSON-RPC messages of the CDP into strongly typed Rust structs, eliminating an entire class of runtime errors associated with protocol mismatches.[^3]

---

## 2. Theoretical Framework: Defining "Fast"

Before writing code, we must rigorously define the metrics of success. The "Time to Complete" mentioned in the user requirements is a composite metric. In SOTA web performance, this is deconstructed into three pillars: **Loading**, **Interactivity**, and **Visual Stability**.

### 2.1 The Taxonomy of Latency

Traditional metrics like `window.onload` often occur late in the page lifecycle, missing the user's perception of speed entirely. In contrast, modern metrics are distributed across the user's journey:

| Metric | What It Measures | Category |
|--------|------------------|----------|
| **LCP** (Largest Contentful Paint) | Visibility of main content | Loading |
| **INP** (Interaction to Next Paint) | Latency of input responses | Interactivity |
| **CLS** (Cumulative Layout Shift) | Visual stability during load | Visual Stability |

Crucially, LCP stops being reported once the user interacts with the page, making it a strict measure of the initial view.

For operations "users appreciate," such as clicking a button, we focus on **Interaction to Next Paint (INP)**. This metric measures the latency of input responses—specifically, the time from the physical hardware event (keypress/click) to the next visual frame update. This captures:

1. **Input Delay** - waiting for the main thread
2. **Processing Time** - event handlers executing
3. **Presentation Delay** - browser layout/paint

A benchmarking harness must maintain active listeners across all these phases—loading, idle, and interaction—to capture the full picture.[^4]

### 2.2 The RAIL Model

Google's RAIL model (Response, Animation, Idle, Load) provides the latency budgets that our harness must validate:

| Category | Budget | Description |
|----------|--------|-------------|
| **Response** | < 100ms | Interactions should be acknowledged |
| **Animation** | ~16ms | 60fps means each frame has ~16ms |
| **Idle** | 50ms chunks | Work should be chunked to allow high-priority interrupts |
| **Load** | 1000-5000ms | Content should be usable depending on network quality |

Our Rust harness will be designed to measure against these specific thresholds, treating a violation not just as "slow" but as a regression failure in the CI/CD pipeline.[^7]

---

## 3. Architecture of the Rust Harness

The architecture of a high-performance benchmarking tool must balance ease of use (simple API) with the raw power needed to orchestrate complex browser behaviors. We utilize the Actor Model implicitly provided by Rust's async runtime, isolating the browser management from the test execution logic.

### 3.1 The chromiumoxide Foundation

The `chromiumoxide` crate serves as the bridge between our Rust code and the Chromium instance. Unlike `headless_chrome`, which relies on synchronous blocking calls, `chromiumoxide` is built on futures and tokio. This is a critical distinction for benchmarking. When we issue a command like "Click Button," we do not simply want to block until the click is done; we often need to simultaneously listen for a `Network.requestWillBeSent` event to verify that the click triggered an API call, or a `Log.entryAdded` event to catch a console error. The async nature of `chromiumoxide` allows us to compose these futures using `tokio::select!` or `futures::join!`, enabling complex, non-blocking orchestration.[^3]

The crate is architected into three layers:

```
┌─────────────────────────────────────────────────────────────┐
│  HIGH-LEVEL API                                             │
│  Browser, Page, Element - ergonomic methods                 │
├─────────────────────────────────────────────────────────────┤
│  PROTOCOL LAYER                                             │
│  Auto-generated Rust structs from CDP PDL                   │
│  Compiler-enforced type safety for all commands             │
├─────────────────────────────────────────────────────────────┤
│  TRANSPORT LAYER                                            │
│  WebSocket connection to browser debugging port             │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Threading Model and Runtime

We leverage the `tokio` runtime for its robust task scheduling:

```rust
// Conceptual architecture
┌─────────────────────────────────────────────────────────────┐
│                    MAIN TEST THREAD                          │
│  - Test logic execution                                      │
│  - Statistical analysis                                      │
│  - Communicates via channels                                 │
├─────────────────────────────────────────────────────────────┤
│                    HANDLER TASK (tokio::spawn)               │
│  - Owns WebSocket stream                                     │
│  - Drives CDP event loop                                     │
│  - Dispatches to oneshot channels                            │
└─────────────────────────────────────────────────────────────┘
```

This separation ensures that even if the test logic performs a heavy computation (e.g., calculating the standard deviation of 10,000 latency samples), the WebSocket heartbeat remains active, preventing the browser from timing out—a common failure mode in synchronous harnesses.[^3]

### 3.3 Context Isolation Strategy

A naive benchmarking approach involves restarting the browser process for every test case to ensure a clean state. However, the startup cost of a Chromium process is measured in seconds, which is unacceptable for a high-throughput suite. SOTA harnesses utilize **Browser Contexts (Incognito Profiles)**.

| Approach | Startup Time | Isolation Level |
|----------|--------------|-----------------|
| New Process | ~2-5 seconds | Complete |
| New Browser Context | ~50-100ms | Logical (Cookies, Storage, Cache) |

**Implementation:**
- **Mechanism:** `Browser::create_browser_context`
- **Benefit:** Creates a logically isolated partition within the same OS process
- **Performance:** Context creation takes milliseconds

Our harness will initialize one global `Browser` instance and then spawn a fresh `BrowserContext` for every single iteration of the benchmark, ensuring perfectly isolated "cold" starts without the process overhead.[^1]

---

## 4. Implementation Phase 1: The Foundation (Loading Performance)

The first requirement of our harness is to measure the initial load performance—specifically LCP and CLS. This phase establishes the core pattern of **"Inject -> Navigate -> Measure."**

### 4.1 The Injection Pattern

While CDP provides some raw metrics, the most accurate way to measure Core Web Vitals is to use the same logic that Google uses in its CrUX report: the `web-vitals` JavaScript library. We cannot rely on the page to have this library installed. Therefore, the harness must inject it.[^11]

We utilize the `Page.addScriptToEvaluateOnNewDocument` command. This powerful CDP feature ensures that our JavaScript payload is executed immediately after the context is created, but before any of the page's own scripts run. This is essential for capturing early lifecycle events that might fire before a traditional `<script>` tag could load.[^12]

```rust
// Conceptual Rust implementation for injection
let script_source = include_str!("./assets/web-vitals.iife.js");
let injection_cmd = chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams::builder()
    .source(script_source)
    .run_immediately(true)
    .build();
page.execute(injection_cmd).await?;
```

### 4.2 The Reporting Pipeline

Once the library is injected, we need a mechanism to exfiltrate the data back to Rust. We configure the `web-vitals` library to report metrics via `console.log` with a unique prefix, such as `__BENCHMARK_METRIC__:`.

On the Rust side, we set up an event listener for `Runtime.consoleAPICalled`. This listener runs concurrently with the page load. When it detects a log message starting with our prefix, it parses the JSON payload into a Rust `Metric` struct.[^13]

```rust
#[derive(Debug, Deserialize)]
struct WebVitalMetric {
    name: String,   // "LCP", "CLS", "INP"
    value: f64,     // The raw value (ms or score)
    delta: f64,     // The change since last report
    id: String,     // Unique interaction ID
}
```

This pipeline provides a robust, asynchronous stream of performance data. Unlike polling `window.performance` at the end of a test, this streaming approach captures metric updates in real-time, allowing us to detect layout shifts that occur dynamically during the load process.[^5]

### 4.3 Navigation and Synchronization

Accurate benchmarking requires precise definition of "Load." The `page.goto()` method typically resolves when the `load` event fires. However, for modern applications, the `load` event is often irrelevant. The SOTA standard is **Network Quiescence** or **Network Idle**.

We implement a custom synchronization primitive in Rust:

```rust
// Pseudocode for Network Idle detection
struct NetworkIdleWaiter {
    active_requests: AtomicUsize,
    cooldown_duration: Duration,
}

impl NetworkIdleWaiter {
    async fn wait_for_idle(&self, page: &Page) -> Result<()> {
        // Listen to Network.requestWillBeSent -> increment counter
        // Listen to Network.loadingFinished -> decrement counter
        // Wait for counter == 0 for `cooldown_duration`
    }
}
```

The "Load" is considered complete only when this counter remains at zero for a defined cooldown period (e.g., 500ms). This ensures that we capture the performance of lazy-loaded components and asynchronous hydration logic that triggers after the initial DOM content is loaded.[^1]

---

## 5. Implementation Phase 2: Interactivity (INP & User Flows)

The user's request specifically highlights the need to measure "operations that users of websites appreciate," such as the time it takes for a dashboard to update after clicking a button. This moves us beyond passive loading metrics into the realm of **Interaction to Next Paint (INP)** and custom user timings.

### 5.1 Scripting User Flows

A "benchmark" in this context is a scripted sequence of actions. We structure these as a `Scenario` object:

```rust
pub struct Scenario {
    pub name: String,
    pub url: String,
    pub steps: Vec<BenchmarkStep>,
}

pub enum BenchmarkStep {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, text: String },
    WaitFor { condition: WaitCondition },
    MeasureCustom { name: String, start_mark: String, end_mark: String },
}

pub enum WaitCondition {
    NetworkIdle,
    Selector(String),
    Timeout(Duration),
}
```

- **Selector Strategy:** Robust CSS selectors via `page.find_element(selector)`
- **Action Execution:** `element.click()` or `element.type_str()` wrap `Input.dispatchMouseEvent` and `Input.dispatchKeyEvent`, simulating "trusted" events[^3]

### 5.2 Measuring Complex Operations (Custom Timings)

For business-specific operations (e.g., "Add to Cart"), standard Web Vitals are insufficient. We need to measure the duration between "User Click" and "UI Updated." We achieve this by bridging the browser's **User Timing API** with our harness:

```
┌──────────────────────────────────────────────────────────────┐
│  1. MARK START                                               │
│     Rust executes: performance.mark('interaction-start')     │
├──────────────────────────────────────────────────────────────┤
│  2. PERFORM ACTION                                           │
│     Rust awaits: element.click()                             │
├──────────────────────────────────────────────────────────────┤
│  3. WAIT FOR EFFECT                                          │
│     Rust awaits: DOM condition OR network condition          │
├──────────────────────────────────────────────────────────────┤
│  4. MARK END                                                 │
│     Rust executes: performance.mark('interaction-end')       │
├──────────────────────────────────────────────────────────────┤
│  5. MEASURE                                                  │
│     Rust retrieves: performance.measure(...)                 │
└──────────────────────────────────────────────────────────────┘
```

This approach gives us a high-precision, business-logic-aware metric that correlates directly with the user's perception of "speed" for that specific feature.

### 5.3 Deconstructing INP

When the harness performs a `click()`, the `web-vitals` library (injected in Phase 1) captures the INP metric. The INP metric is a composite sum of three distinct phases:

| Phase | Description | Diagnostic Implication |
|-------|-------------|------------------------|
| **Input Delay** | Time blocked before handling click | Main-thread congestion from background scripts |
| **Processing Time** | Time executing event handlers | Inefficient application code |
| **Presentation Delay** | Time calculating layout and painting | Expensive CSS or large DOM updates |

By reporting these sub-metrics, our harness becomes a **diagnostic tool**, not just a scoreboard.[^5]

---

## 6. Implementation Phase 3: The SOTA Frontier (Trace Analysis)

For the most advanced use cases ("SOTA benchmarking knowledge"), simple metric collection is not enough. We need to know **why** a metric is slow. This requires integrating the Chrome Tracing ecosystem.

### 6.1 The Tracing Domain

The CDP `Tracing` domain allows us to record a profile of the browser's execution—the same data used to generate the "Performance" tab in DevTools.

```rust
// Tracing workflow
async fn capture_trace(page: &Page) -> Result<Vec<TraceEvent>> {
    // 1. Start Tracing
    let categories = vec![
        "blink.user_timing",
        "devtools.timeline",
        "v8.execute",
        "latencyInfo",
    ];
    page.execute(Tracing::start().categories(categories)).await?;

    // 2. Perform operations...

    // 3. Stop Tracing
    page.execute(Tracing::end()).await?;

    // 4. Collect data via Tracing.dataCollected events
    collect_trace_chunks().await
}
```

### 6.2 Parsing Traces in Rust

The trace data format is a complex JSON array of event objects.[^16] While we could dump this to a file for manual inspection in [ui.perfetto.dev](https://ui.perfetto.dev)[^17], a truly automated harness parses this data on the fly to detect performance anti-patterns.

```rust
#[derive(Debug, Deserialize)]
struct TraceEvent {
    name: String,
    cat: String,      // Category
    ph: String,       // Phase (B=begin, E=end, X=complete)
    ts: u64,          // Timestamp (microseconds)
    dur: Option<u64>, // Duration (for X events)
    pid: u32,         // Process ID
    tid: u32,         // Thread ID
    args: Option<serde_json::Value>,
}

impl TraceEvent {
    fn is_long_task(&self) -> bool {
        self.name == "FunctionCall"
            && self.dur.unwrap_or(0) > 50_000 // > 50ms
    }

    fn is_layout_shift(&self) -> bool {
        self.name == "LayoutShift"
    }
}
```

**Capabilities:**
- Scan for `LayoutShift` events to calculate exact screen coordinates of unstable elements
- Scan for `FunctionCall` events on the main thread exceeding 50ms (**Long Tasks**)
- Identify specific JavaScript functions causing performance bottlenecks

This capability transforms the tool from a passive observer into an **active quality gate**.[^16]

---

## 7. Simulation Fidelity: Throttling & Emulation

A benchmark running on a developer's high-end workstation connected to gigabit fiber is a "best-case scenario" that rarely reflects the reality of the user base. To make our measurements representative, we must **degrade the environment**.

### 7.1 Network Throttling

We use the browser's internal network emulation via CDP—more deterministic and easier to configure than external proxies.

**Command:** `Network.emulateNetworkConditions`

| Profile | Download | Upload | Latency |
|---------|----------|--------|---------|
| Fast 3G | 1.6 Mbps | 750 Kbps | 150ms |
| Slow 4G | 3.0 Mbps | 1.5 Mbps | 100ms |
| Offline | 0 | 0 | 0 |

```rust
// Network throttling configuration
let conditions = NetworkConditions {
    offline: false,
    latency: 150.0,      // ms
    download_throughput: 1.6 * 1024.0 * 1024.0 / 8.0, // bytes/sec
    upload_throughput: 750.0 * 1024.0 / 8.0,
};
page.execute(Network::emulate_network_conditions(conditions)).await?;
```

Advanced SOTA benchmarking also involves simulating **packet loss or jitter**, which CDP supports.[^14]

### 7.2 CPU Throttling

Mobile devices typically possess CPUs that are 4x to 6x slower than a desktop processor. While network throttling affects load time, CPU throttling affects interactivity (INP) and hydration speed.

**Command:** `Emulation.setCPUThrottlingRate`

| Rate | Simulates |
|------|-----------|
| 1 | No throttling |
| 4 | Mid-tier mobile device (industry standard) |
| 6 | Low-end mobile device |

```rust
// CPU throttling
page.execute(Emulation::set_cpu_throttling_rate(4.0)).await?;
```

---

## 8. Statistical Rigor: Moving Beyond Averages

In the domain of web performance, the "Average" (Arithmetic Mean) is a **dangerous metric**. Latency data follows a log-normal distribution, characterized by a "long tail" of slow experiences.

### 8.1 The "Long Tail" Reality

```
Distribution of Page Load Times (conceptual)

  Frequency
     │
     │  ▓▓▓▓
     │  ▓▓▓▓▓▓▓
     │  ▓▓▓▓▓▓▓▓▓▓
     │  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
     │  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░
     └────────────────────────────────────────────────────────▶
        P50 (Median)     P95            P99        Time (ms)
                                  ▲
                            "Long Tail"
```

A site might have an average load time of 2 seconds, but if 10% of users experience 15 seconds, the average hides a significant reliability problem. SOTA benchmarking focuses almost exclusively on **higher percentiles**:

| Percentile | Purpose |
|------------|---------|
| P50 (Median) | "Typical" user experience |
| P75 | Performance budget target |
| P95 | "Frustrated" user threshold |
| P99 | Reliability SLA |

### 8.2 Implementing Robust Statistics in Rust

To achieve statistical significance, a single test run is insufficient. The browser is a noisy environment.

**Configuration Parameters:**

| Parameter | Default | Purpose |
|-----------|---------|---------|
| `iterations` | 30+ | Minimum runs for significance |
| `warmup` | 2 | Runs to discard (cold start penalty) |
| `outlier_method` | IQR | Interquartile Range detection |

**Outlier Detection (IQR Method):**

```rust
fn detect_outliers(samples: &[f64]) -> Vec<usize> {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let q1 = percentile(&sorted, 25.0);
    let q3 = percentile(&sorted, 75.0);
    let iqr = q3 - q1;

    let lower_fence = q1 - 1.5 * iqr;
    let upper_fence = q3 + 1.5 * iqr;

    samples.iter()
        .enumerate()
        .filter(|(_, &v)| v < lower_fence || v > upper_fence)
        .map(|(i, _)| i)
        .collect()
}
```

In a CI context, outliers should not just be ignored; they should be reported as **"Stability Failures"**—flakiness in the application's performance profile.[^22]

Rust crates for statistics:
- `statrs` - Statistical distributions and functions[^24]
- `average` - Online algorithms for mean, variance[^21]

---

## 9. Integration and Configuration

### 9.1 Configuration Schema

To make the harness reusable across different projects, it should be **data-driven**:

```toml
# benchmark.toml

[benchmark]
name = "Checkout Flow"
url = "https://staging.example.com/cart"
iterations = 50
warmup = 3

[throttling]
profile = "Slow 4G"
cpu_slowdown = 4.0

[thresholds]
lcp_p95 = 2500  # ms
inp_p95 = 200   # ms
cls_p95 = 0.1   # score

[[interactions]]
name = "Click Checkout"
selector = "button.checkout-btn"
wait_for = "network_idle"

[[interactions]]
name = "Complete Purchase"
selector = "button.confirm-purchase"
wait_for = { selector = ".order-confirmation" }
```

### 9.2 CI/CD Pipeline Integration

The harness is designed to run in headless mode within Docker containers on CI platforms:

```yaml
# .github/workflows/benchmark.yml
name: Performance Benchmarks

on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Chrome
        run: |
          wget -q https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
          sudo dpkg -i google-chrome-stable_current_amd64.deb

      - name: Run Benchmarks
        run: cargo run --release -p benchmark-harness

      - name: Upload Results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: benchmark-results.json
```

**Exit Code Strategy:**
- Exit 0: All thresholds met
- Exit 1: Threshold violation (blocks merge)
- Exit 2: Stability failure (outlier detection triggered)

The harness generates structured JSON reports for historical tracking and trend analysis.[^25]

---

## 10. Conclusion

By building a custom benchmarking harness in Rust with `chromiumoxide`, we gain a level of control and insight that off-the-shelf tools cannot match. We move from the opaque "black box" testing of the past to a transparent, highly instrumented analysis of the modern web experience:

| Capability | Benefit |
|------------|---------|
| **Throttling Simulation** | Simulate harsh reality of mobile devices |
| **INP Monitoring** | Quantify tactile latency of interactions |
| **Trace Analysis** | Understand *why* metrics are slow |
| **Statistical Rigor** | Outlier detection and percentile-based assertions |
| **Parallel Contexts** | High-throughput benchmarking without process overhead |

This infrastructure does not just measure speed; it enforces an engineering culture where **performance is a first-class citizen**—visible, quantifiable, and non-negotiable.

---

## References

[^1]: chromiumoxide - Rust - Docs.rs, https://docs.rs/chromiumoxide
[^2]: How to automatically Profile Performance of Rust Applications - Paweł Urbanek, https://pawelurbanek.com/rust-optimize-performance
[^3]: mattsse/chromiumoxide: Chrome Devtools Protocol rust API - GitHub, https://github.com/mattsse/chromiumoxide
[^4]: Measure And Optimize Google Core Web Vitals: A Guide | DebugBear, https://www.debugbear.com/docs/metrics/core-web-vitals
[^5]: Interaction to Next Paint (INP) | Articles - web.dev, https://web.dev/articles/inp
[^6]: Core Web Vitals — What they are and how to optimize them - Adobe for Business, https://business.adobe.com/blog/basics/web-vitals-explained
[^7]: Analyze runtime performance | Chrome DevTools, https://developer.chrome.com/docs/devtools/performance
[^8]: What is the difference between this and the headless_chrome crate? | Hacker News, https://news.ycombinator.com/item?id=25418154
[^9]: chromiumoxide - Rust - Docs.rs (spider_chrome), https://docs.rs/spider_chrome
[^10]: chromiumoxide/src/browser.rs at main - GitHub, https://github.com/mattsse/chromiumoxide/blob/main/src/browser.rs
[^11]: GoogleChrome/web-vitals: Essential metrics for a healthy site - GitHub, https://github.com/GoogleChrome/web-vitals
[^12]: Analyzing anti-detect browsers: How to detect scripts injected via CDP in Chrome, https://blog.castle.io/how-to-detect-scripts-injected-via-cdp-in-chrome-2/
[^13]: Listen for console message with chromiumoxide - Stack Overflow, https://stackoverflow.com/questions/77574645/listen-for-console-message-with-chromiumoxide
[^14]: Response in chromiumoxide::cdp::browser_protocol::network - Rust - Docs.rs, https://docs.rs/spider_chrome/latest/chromiumoxide/cdp/browser_protocol/network/struct.Response.html
[^15]: The Trace Event Profiling Tool (about:tracing), https://www.chromium.org/developers/how-tos/trace-event-profiling-tool/
[^16]: Trace Event Format - Google Docs, https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview
[^17]: What is Perfetto? - Perfetto Tracing Docs, https://perfetto.dev/docs/
[^18]: Network Throttling in Chrome DevTools - DebugBear, https://www.debugbear.com/blog/chrome-devtools-network-throttling
[^19]: How to Perform Network Throttling in Chrome to Simulate Poor Network - BrowserStack, https://www.browserstack.com/guide/how-to-perform-network-throttling-in-chrome
[^20]: CPU Throttling In Chrome DevTools and Lighthouse - DebugBear, https://www.debugbear.com/blog/cpu-throttling-in-chrome-devtools-and-lighthouse
[^21]: average - Rust - Docs.rs, https://docs.rs/average
[^22]: Analysis Process - Criterion.rs Documentation, https://bheisler.github.io/criterion.rs/book/analysis.html
[^23]: What About Warmup? - Engineering Blog - AppFolio, https://engineering.appfolio.com/appfolio-engineering/2017/5/2/what-about-warmup
[^24]: statrs - Rust - Docs.rs, https://docs.rs/statrs/
[^25]: How to build a Custom Benchmarking Harness in Rust - Bencher, https://bencher.dev/learn/benchmarking/rust/custom-harness/
[^26]: How to catch performance regressions in Rust - Reddit, https://www.reddit.com/r/rust/comments/11xhwv3/how_to_catch_performance_regressions_in_rust/
