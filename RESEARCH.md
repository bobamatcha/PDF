# Architecting the Local-First Modular Monolith

> A Strategic Integration Plan for agentPDF, Web, Corpus, and DocSign

## Executive Summary

The transition from disparate functional prototypes to a unified, production-grade application represents one of the most precarious phases in software development. For a high-velocity developer aiming to launch a local-first infrastructure product, the architectural decisions made at this juncture define not only the immediate time-to-market but also the long-term maintainability and scalability of the system.

The original intent to integrate the components—agentPDF (automation), Web (interface), Corpus (search), and DocSign (identity)—into a microservices architecture reflects a desire for modularity and separation of concerns. However, rigorous analysis of the current landscape of Rust development, local-first patterns, and the constraints of a solo or small-team environment suggests that a distributed microservices architecture is a strategic misstep that will introduce unnecessary latency, operational complexity, and deployment fragility.

**This report proposes a refined architectural strategy: the Local-First Modular Monolith.**

By leveraging the sophisticated workspace capabilities inherent in the Rust ecosystem, specifically Cargo Workspaces, it is possible to achieve the logical isolation of microservices without incurring the physical penalties of distributed systems. This approach unifies the four components into a single, high-performance binary deployed to the user's device via the Tauri framework. It integrates Tantivy for embedded, millisecond-latency search, adopts PAdES standards for legally robust digital signatures, and utilizes rspc to guarantee end-to-end type safety between the Rust backend and the TypeScript frontend.

The following analysis details a phased integration plan designed to minimize technical debt while maximizing launch velocity. It deconstructs the technical implementation of embedding heavy computational tasks—such as cryptographic signing and full-text indexing—within a responsive user interface, ensuring that the application remains performant and robust. This strategy prioritizes the "ASAP" launch requirement by removing the need for complex network orchestration, while simultaneously laying the groundwork for future scalability through a rigorous data synchronization roadmap.

---

## 1. Architectural Paradigm Shift: The Case for the Modular Monolith

### 1.1 The Trap of Premature Microservices

The industry discourse surrounding software architecture has heavily favored microservices for the past decade, often conflating modularity with distributed systems. In the context of large enterprises like Netflix or Amazon, microservices serve to decouple hundreds of engineering teams, allowing them to deploy independently. However, for a single expert developer or a compact team, adopting this pattern introduces what is known as the **"Distributed Monolith"**. This anti-pattern manifests when tightly coupled components are separated by network boundaries, resulting in a system that retains all the dependencies of a monolith but incurs the operational costs of a distributed system.

If agentPDF, Web, Corpus, and DocSign were to be implemented as distinct networked services, the application would suffer from significant performance and reliability drawbacks:

- A user-initiated search query traveling from the Web interface to the Corpus service would require:
  1. Serialization of the request into JSON
  2. Transmission over a local network loopback
  3. Deserialization by the service
  4. Execution of the query
  5. Re-serialization of the results
  6. Final transmission back to the interface

In a local-first application written in Rust, this interaction should be a **direct function call** involving zero-copy memory access, executing in microseconds rather than milliseconds.

Furthermore, the operational overhead of managing four separate process lifecycles, health checks, and inter-service failure modes—such as the DocSign daemon crashing while the Web interface remains active—creates a fragile user experience that requires complex remediation code.

### 1.2 The Modular Monolith Alternative

The strategic refinement proposed here is the adoption of a **Modular Monolith** architecture. This pattern uses Rust's Cargo Workspace feature to maintain strict logical boundaries between components while compiling them into a single deployable unit.

The directory structure is organized into a "meta-repo" where distinct crates reside in a `crates/` directory, and the user-facing application resides in `apps/`. Each component—Corpus, DocSign, and agentPDF—maintains its own `Cargo.toml` file, defining its specific dependencies and public API surface. This ensures that while the code is co-located, the logical separation prevents the "spaghetti code" often associated with traditional monoliths.

**Benefits:**

- **Launch Velocity**: Eliminates the need for network orchestration, service discovery, and complex deployment pipelines
- **Shared Dependencies**: Libraries like `tokio` for async runtime or `serde` for serialization can be compiled once and shared across all components, drastically reducing build times and binary size
- **Future-Proof**: Should the Corpus search engine need to scale independently in the future, the clean crate boundary allows it to be wrapped in a lightweight web server layer (like Axum or Actix) and extracted into a microservice with minimal refactoring

### 1.3 Adhering to Local-First Principles

The integration plan is fundamentally guided by the **"Seven Ideals for Local-First Software"**:

1. **No Spinners**: The primary ideal relevant to this architectural pivot. In a distributed architecture, network latency makes spinners inevitable. In the proposed monolithic architecture, interactions with the Corpus and DocSign engines occur in-process, allowing for instantaneous feedback.

2. **Data Ownership**: The user's work is not trapped in a remote server but resides on their local device, managed by local SQLite databases and file systems.

3. **Network Optional**: The network remains an optional enhancement rather than a critical dependency, providing a robust offline experience that is essential for a productivity tool handling sensitive documents.

---

## 2. The Core Framework: Tauri & Rust Workspace Integration

### 2.1 The Meta-Repo Structure

To integrate the four existing codebases while preserving their development history, the new repository functions as a workspace container. The migration process involves using `git merge --allow-unrelated-histories` to import the distinct repositories into the new workspace structure. This preserves the git blame history, which is critical for understanding the evolution of the code and debugging regressions.

**Recommended Structure:**

```
product-workspace/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── corpus/             # Search engine
│   ├── docsign/            # Signing engine
│   ├── agentpdf-core/      # Shared types
│   └── compliance/         # Rule engine
└── apps/
    ├── web/                # Tauri application
    └── cli/                # Command-line tools
```

The `Cargo.toml` file at the root defines the workspace members, allowing Cargo to resolve dependencies across the entire project graph. This setup facilitates a unified build pipeline where a single command, `cargo build`, compiles the entire suite.

### 2.2 Tauri as the Application Runtime

The Web component is transformed from a standard web application into a desktop application using the Tauri framework. Tauri differs from other frameworks like Electron by not bundling a runtime like Node.js. Instead, it relies on the operating system's native WebView (WebView2 on Windows, WebKit on macOS and Linux) and communicates with a Rust backend.

**Advantages:**
- Significantly smaller binary sizes
- Lower memory footprints
- Native performance

### 2.3 Type-Safe Communication with rspc

To ensure rigorous type safety and prevent runtime errors caused by mismatched API signatures, the plan integrates **rspc (Rust Server Procedure Call)**. rspc allows for the definition of backend procedures in Rust and automatically generates TypeScript bindings for the frontend.

This means that any change to a function signature in the Corpus or DocSign crates will immediately trigger a compile-time error in the frontend build process, providing a level of safety that manual IPC handling cannot match.

### 2.4 The Async Runtime and Event Loop

The Rust backend operates on an asynchronous runtime, typically provided by `tokio`. This is crucial for maintaining a responsive user interface. Heavy computational tasks, such as generating a cryptographic signature in DocSign or indexing a large PDF in Corpus, must not block the main thread responsible for handling IPC messages from the frontend.

**Strategy:**
- Use `tokio::task::spawn_blocking` to offload CPU-intensive operations to a dedicated thread pool
- Spawn persistent async tasks during application setup for long-running background tasks
- Communicate updates to the frontend via Tauri's event emission system

---

## 3. Component Strategy: Corpus (The Embedded Search Engine)

### 3.1 Moving Beyond External Services

The Corpus component, responsible for document indexing and retrieval, represents a significant integration challenge. Traditional web architectures might employ an external search service like Elasticsearch or Meilisearch. However, running a separate search process violates the local-first, single-binary requirement.

**Solution: Embed the search engine directly using Tantivy.**

Tantivy is a high-performance full-text search engine library written in Rust, strongly inspired by Apache Lucene. It supports advanced features such as:
- BM25 scoring
- Faceted search
- Boolean queries

Most importantly, it operates entirely within the application's process memory, eliminating the overhead of IPC and HTTP transport layers found in server-client search architectures.

### 3.2 Concurrency and State Management

Embedding a search engine requires careful management of concurrent access to the index:

- **IndexWriter**: Single-threaded, requires exclusive access for writing
- **IndexReader**: Can support multiple concurrent search threads

**Pattern:**
```rust
Arc<RwLock<IndexWriter>>
```

This setup allows:
- Multiple search queries to execute in parallel by acquiring a read lock
- Write operations to be batched or scheduled to minimize lock duration
- User's ability to search the corpus is never blocked by background indexing

---

## 4. Component Strategy: DocSign (Cryptographic Identity)

### 4.1 Digital Signatures vs. Electronic Signatures

It is imperative to distinguish between:

| Type | Description |
|------|-------------|
| **Electronic Signature** | An image of a signature placed on a PDF |
| **Digital Signature** | A cryptographic proof of integrity |

The integration plan targets the implementation of **PAdES (PDF Advanced Electronic Signatures)**, the standard for secure PDF signing in Europe and increasingly globally.

PAdES builds upon the ISO 32000-1 PDF specification to allow for long-term validation (LTV) of signatures. This ensures that a signature remains valid even if the signing certificate expires, provided that revocation information (OCSP/CRL) was embedded at the time of signing.

### 4.2 Implementation Strategy: Gradual Compliance

Given the "launch ASAP" constraint, attempting a full PAdES-LTV implementation immediately creates a high risk of project stall.

**Recommended Phased Approach:**

| Phase | Implementation | Capability |
|-------|----------------|------------|
| V1 | Basic Digital Signatures (CMS/PKCS#7) | Core cryptographic assurance |
| V2 | Hardware token support (PKCS#11) | Smart card integration |
| V3 | Full PAdES-LTV compliance | Long-term validation |

**Architecture:**
```rust
pub trait SigningStrategy {
    fn sign(&self, document: &[u8]) -> Result<SignedDocument>;
}
```

This abstraction allows the application to support different signing methods interchangeably.

### 4.3 Handling PDF Structure

Technically, adding a digital signature to a PDF is an **incremental update**. The original file content is left untouched, and a new revision is appended containing:
- The signature dictionary
- The byte range it covers

Libraries:
- **lopdf**: Low-level PDF object manipulation (dictionaries, streams, cross-reference tables)
- **pdf-sign**: Higher-level abstraction for signing (supports OpenPGP and Sigstore)

---

## 5. Component Strategy: agentPDF (The Automation Worker)

### 5.1 Background Processing Architecture

agentPDF represents the autonomous worker of the system—the component that "does the work" while the user is away:
- Monitoring folders for new invoices
- Performing OCR on scanned documents
- Automatically organizing files

**Integration Strategy:**
1. Refactor the agentPDF entry point into a library function: `run_watcher()`
2. Spawn during Tauri application setup
3. Leverage the `notify` crate for file system events
4. Communicate results via event emission

### 5.2 OCR and Heavy Compute

**Options:**

| Library | Type | Notes |
|---------|------|-------|
| Tesseract | External binary | Traditional standard, complicates single-binary distribution |
| ocrs | Pure Rust | ML-based, embeddable, aligns with local-first philosophy |

**Fallback Pattern:**
If agentPDF logic is too heavy or relies on legacy non-Rust code, Tauri allows bundling as a separate binary (sidecar) managed by the main application.

---

## 6. The Unifying Interface: Web (Frontend Integration)

### 6.1 Type-Safe Communication with rspc

The Web component serves as the presentation layer, strictly decoupled from business logic. The frontend should contain no complex data processing code; its role is to:
- Display state
- Capture user intent

rspc integration eliminates a common class of bugs where the backend API changes and the frontend breaks at runtime. Such changes cause build-time errors, forcing reconciliation before the code can run.

### 6.2 State Management and Optimistic UI

Local-first applications require **"Optimistic UI"**:

1. User action (like signing a document) triggers immediately
2. UI updates optimistically, assuming success
3. Mutation sent to Rust backend asynchronously
4. On success: query cache invalidated or updated
5. On failure: UI reverts to previous state with error display

**Implementation:**
- TanStack Query (React Query) for state management
- rspc integration provides typed hooks for queries and mutations

---

## 7. Data Persistence & Synchronization Strategy

### 7.1 SQLite as the Foundation

SQLite is the de facto standard for local-first software:
- Serverless
- Reliable
- Supports concurrent access via Write-Ahead Logging (WAL) mode

**Principle:** All database interactions occur within the Rust backend. The frontend invokes high-level commands, and the backend translates these into optimized SQL queries.

### 7.2 The Path to Synchronization

**Phase 1: Local SQLite**
- Standard normalized schema
- Local `.db` file storage
- Meets immediate MVP requirement

**Phase 2: Operation Log**
- Introduce "Operation Log" or "Action Log" table
- Record intent as immutable log entries (Event Sourcing pattern)
- Log serves as source of truth; current state derived by replay

**Phase 3: CRDT Synchronization**
- Activate sync engine for Operation Log replication
- Leverage Automerge or CRDTs for conflict-free merging
- Sync operations rather than final state

---

## 8. Operational Strategy: Testing, CI/CD, and Release

### 8.1 Unified Testing Pipeline

The Modular Monolith structure simplifies testing significantly:

```bash
# Single command runs all tests
cargo test
```

- Unit tests across all crates (Corpus, DocSign, agentPDF)
- Integration tests simulating user flows
- No complex test harnesses to spin up multiple services

### 8.2 Deployment and Distribution

**Process:**
1. Configure GitHub Actions for build matrix (Windows, macOS, Linux)
2. Compile Rust binary
3. Bundle frontend assets
4. Package into platform-specific installers (.msi, .dmg, .deb)
5. Code-sign released binaries

---

## 9. Detailed Execution Roadmap

### Phase 1: The "Grand Unification" (Weeks 1-2)

**Objective:** Establish single workspace repository and achieve successful compilation.

- [ ] Initialize product-workspace repository
- [ ] Import agentPDF, Corpus, and DocSign into `crates/` using `git merge --allow-unrelated-histories`
- [ ] Create workspace `Cargo.toml` and unify common dependencies
- [ ] Scaffold Tauri application in `apps/web`
- [ ] Configure rspc router with basic endpoint

### Phase 2: Embedding the Core (Weeks 3-4)

**Objective:** Enable core functionality—search and signing—within new architecture.

- [ ] Refactor Corpus to expose `SearchEngine` struct
- [ ] Integrate into Tauri state management
- [ ] Refactor DocSign to expose `SigningStrategy` trait
- [ ] Implement basic V1 (local key) strategy
- [ ] Implement `sign_document` async command
- [ ] Build frontend "Document View"
- [ ] Integrate search and sign features using rspc hooks

### Phase 3: The Automation Layer (Week 5)

**Objective:** Activate background worker.

- [ ] Refactor agentPDF entry point into `run_watcher()` library function
- [ ] Configure Tauri setup hook to spawn watcher task
- [ ] Implement event emission logic for frontend notifications

### Phase 4: Polish & Launch (Week 6)

**Objective:** Prepare for public release.

- [ ] Configure Tauri updater for over-the-air updates
- [ ] Perform performance testing on Tantivy index and signing process
- [ ] Code-sign application binaries for distribution

---

## 10. Key Technologies Summary

| Component | Technology | Purpose |
|-----------|------------|---------|
| Workspace | Cargo Workspaces | Logical isolation with unified compilation |
| Runtime | Tauri | Native desktop app with web UI |
| Type Safety | rspc | End-to-end TypeScript/Rust type safety |
| Search | Tantivy | Embedded full-text search |
| Signatures | PAdES (lopdf, pdf-sign) | Legal digital signatures |
| Async | Tokio | Non-blocking I/O and background tasks |
| Database | SQLite + sqlx | Local-first persistence |
| State | TanStack Query | Optimistic UI with cache management |

---

## Conclusion

The shift from a distributed microservices architecture to a Local-First Modular Monolith is not a retreat from sophistication; it is a **strategic advancement** towards reliability, performance, and velocity.

By exploiting the compile-time guarantees of Rust and the architectural unification of Cargo Workspaces, the proposed plan eliminates entire classes of distributed system failures. It delivers on the "ASAP" launch requirement by significantly reducing the operational surface area, allowing a focus on feature development rather than infrastructure management.

The result is a robust, high-performance desktop application that empowers users with true data ownership and instant responsiveness—a product that is not just a compromise, but a superior realization of local-first principles.

---

## References

1. [Microservices vs. monolithic architecture - Atlassian](https://www.atlassian.com/microservices/microservices-architecture/microservices-vs-monolith)
2. [Monolith vs. Microservices Discussion - Reddit](https://www.reddit.com/r/softwarearchitecture/comments/1eflqzl/monolith_vs_microservices_whats_your_take/)
3. [Microservices vs Monolithic Architecture - Stack Overflow](https://stackoverflow.com/questions/33041733/microservices-vs-monolithic-architecture)
4. [Modular monolith and microservices - Hacker News](https://news.ycombinator.com/item?id=45810482)
5. [Rust Projects with Multiple Entry Points - YouTube](https://www.youtube.com/watch?v=xnAqZcvuEA0)
6. [Building Hybrid Rust and C++ Applications - KDAB](https://www.kdab.com/software-technologies/rust/how-to-build-hybrid-rust-and-c-applications/)
7. [Local-first software - Ink & Switch](https://www.inkandswitch.com/essay/local-first/)
8. [SyncKit – Offline-first sync engine - Hacker News](https://news.ycombinator.com/item?id=46069598)
9. [How to merge multiple Git repos - Graham F. Scott](https://gfscott.com/blog/merge-git-repos-and-keep-commit-history/)
10. [Merge two Git repositories - Stack Overflow](https://stackoverflow.com/questions/13040958/merge-two-git-repositories-without-breaking-file-history)
11. [Multi-crate cargo repositories best practices - Rust Forum](https://users.rust-lang.org/t/multi-crate-cargo-repositories-best-practices/692)
12. [Cargo Workspaces - GitHub](https://github.com/rust-lang/book/issues/1512)
13. [Tauri Architecture](https://tauri.app/v1/references/architecture/)
14. [Tauri - specta-rs (rspc)](https://specta.dev/docs/rspc/integrations/tauri)
15. [Async Tauri Commands - YouTube](https://www.youtube.com/watch?v=uKCwEQlRwQ8)
16. [Tauri Process Model](https://tauri.app/v1/references/architecture/process-model)
17. [Tantivy - GitHub](https://github.com/quickwit-oss/tantivy)
18. [Full-text-search - Simon Willison](https://simonwillison.net/tags/full-text-search/)
19. [Static.wiki SQLite - Hacker News](https://news.ycombinator.com/item?id=28012829)
20. [Tauri State Management](https://v2.tauri.app/develop/state-management/)
21. [PAdES PDF Advanced Electronic Signature - eSignGlobal](https://www.esignglobal.com/blog/pades-standard-pdf-advanced-electronic-signature)
22. [PAdES - Wikipedia](https://en.wikipedia.org/wiki/PAdES)
23. [pdf-sign Rust utility - Reddit](https://www.reddit.com/r/crypto/comments/1pm5kys/modern_pdf_signing_utility_written_in_rust/)
24. [trust_pdf - Docs.rs](https://docs.rs/trust_pdf)
25. [iText signing PDF with smart card - Stack Overflow](https://stackoverflow.com/questions/33019686/itext-signing-pdf-using-external-signature-with-smart-card)
26. [Thread safety in Rust - Reddit](https://www.reddit.com/r/rust/comments/1er9mzw/issue_with_thread_safety/)
27. [ocrs - GitHub](https://github.com/robertknight/ocrs)
28. [Tauri Sidecar - Embedding External Binaries](https://tauri.app/v1/guides/building/sidecar)
29. [React Query Patterns - Medium](https://steven-hankin.medium.com/3-useful-patterns-for-react-query-f3b0c98d77df)
30. [React Query API Design - TkDodo](https://tkdodo.eu/blog/react-query-api-design-lessons-learned)
31. [Distributed SQLite - Reddit](https://www.reddit.com/r/sqlite/comments/1na3pay/distributed_sqlite_with_local_first_approach/)
32. [Building a todo app in Tauri with SQLite](https://tauritutorials.com/blog/building-a-todo-app-in-tauri-with-sqlite-and-sqlx)
33. [Automerge and Convex](https://stack.convex.dev/automerge-and-convex)
