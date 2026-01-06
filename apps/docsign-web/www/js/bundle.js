var y={BUNDLE_START:"docsign:bundle:start",BUNDLE_LOADED:"docsign:bundle:loaded",NAMESPACE_INIT:"docsign:namespace:init",PDFJS_LOAD_START:"docsign:pdfjs:load:start",PDFJS_LOADED:"docsign:pdfjs:loaded",SIG_MODAL_OPEN:"docsign:sig:modal:open",SIG_MODAL_RENDERED:"docsign:sig:modal:rendered",SIG_CAPTURE_INIT:"docsign:sig:capture:init",PDF_RENDER_START:"docsign:pdf:render:start",PDF_RENDER_END:"docsign:pdf:render:end",SIG_APPLIED:"docsign:sig:applied",INTERACTIVE:"docsign:interactive"},fe=class i{constructor(){this.marks=new Map;this.timings=new Map;this.loadingIndicators=new Map;this.enabled=this.determineEnabled()}determineEnabled(){if(typeof window>"u")return!1;try{if(window.location?.search?.includes("perf=1")||typeof localStorage<"u"&&localStorage.getItem("docsign:perf")==="1")return!0}catch{}return!1}static getInstance(){return i.instance||(i.instance=new i),i.instance}setEnabled(e){this.enabled=e;try{typeof localStorage<"u"&&(e?localStorage.setItem("docsign:perf","1"):localStorage.removeItem("docsign:perf"))}catch{}}isEnabled(){return this.enabled}mark(e){let t=performance.now();if(this.marks.set(e,t),this.enabled)try{performance.mark(e)}catch{}}start(e){let t=performance.now();if(this.timings.set(e,{name:e,startTime:t}),this.enabled)try{performance.mark(`${e}:start`)}catch{}}end(e){let t=this.timings.get(e);if(!t)return;let n=performance.now();if(t.endTime=n,t.duration=n-t.startTime,this.enabled){try{performance.mark(`${e}:end`),performance.measure(e,`${e}:start`,`${e}:end`)}catch{}t.duration>100}return t.duration}getDuration(e){return this.timings.get(e)?.duration}getMark(e){return this.marks.get(e)}measureBetween(e,t){let n=this.marks.get(e),r=this.marks.get(t);if(!(n===void 0||r===void 0))return r-n}getMetrics(){let e=this.measureBetween(y.BUNDLE_START,y.INTERACTIVE),t=this.measureBetween(y.PDFJS_LOAD_START,y.PDFJS_LOADED),n=this.measureBetween(y.SIG_MODAL_OPEN,y.SIG_MODAL_RENDERED),r=this.measureBetween(y.PDF_RENDER_START,y.PDF_RENDER_END),o={};this.timings.forEach((l,u)=>{l.duration!==void 0&&(o[u]=l.duration)});let s={};this.marks.forEach((l,u)=>{s[u]=l});let a=[];if(typeof performance<"u"&&performance.getEntriesByType)try{a=[...performance.getEntriesByType("mark"),...performance.getEntriesByType("measure")].filter(l=>l.name.startsWith("docsign:"))}catch{}return{timeToInteractive:e,pdfJsLoadTime:t,signatureModalRenderTime:n,pdfRenderTime:r,timings:o,marks:s,entries:a}}logMetrics(){if(!this.enabled)return;let e=this.getMetrics();if(e.timeToInteractive!==void 0){let t=e.timeToInteractive<200?"OK":"SLOW"}e.pdfJsLoadTime,e.signatureModalRenderTime,e.pdfRenderTime,Object.keys(e.timings).length>0}clear(){if(this.marks.clear(),this.timings.clear(),typeof performance<"u")try{performance.clearMarks(),performance.clearMeasures()}catch{}}showLoading(e,t){let n=`loading-${Date.now()}-${Math.random().toString(36).slice(2,7)}`,r=document.createElement("div");r.id=n,r.className="docsign-loading-overlay",r.setAttribute("role","alert"),r.setAttribute("aria-busy","true"),r.setAttribute("aria-live","polite"),r.innerHTML=`
      <div class="docsign-loading-content">
        <div class="docsign-loading-spinner" aria-hidden="true">
          <svg viewBox="0 0 50 50" width="50" height="50">
            <circle cx="25" cy="25" r="20" fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round">
              <animate attributeName="stroke-dasharray" values="1,150;90,150;90,150" dur="1.5s" repeatCount="indefinite"/>
              <animate attributeName="stroke-dashoffset" values="0;-35;-125" dur="1.5s" repeatCount="indefinite"/>
            </circle>
          </svg>
        </div>
        <p class="docsign-loading-message">${this.escapeHtml(e)}</p>
      </div>
    `,this.injectLoadingStyles();let o=t?document.getElementById(t):document.body;return o&&o.appendChild(r),this.loadingIndicators.set(n,{element:r,message:e,startTime:performance.now()}),n}hideLoading(e){let t=this.loadingIndicators.get(e);if(!t)return 0;let n=performance.now()-t.startTime;return t.element.classList.add("docsign-loading-fade-out"),setTimeout(()=>{t.element.remove()},200),this.loadingIndicators.delete(e),this.enabled&&n>500,n}updateLoadingMessage(e,t){let n=this.loadingIndicators.get(e);if(!n)return;let r=n.element.querySelector(".docsign-loading-message");r&&(r.textContent=t),n.message=t}injectLoadingStyles(){if(document.getElementById("docsign-loading-styles"))return;let e=document.createElement("style");e.id="docsign-loading-styles",e.textContent=`
      .docsign-loading-overlay {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background-color: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100000;
        animation: docsign-loading-fade-in 0.2s ease;
      }

      .docsign-loading-fade-out {
        animation: docsign-loading-fade-out 0.2s ease forwards;
      }

      @keyframes docsign-loading-fade-in {
        from { opacity: 0; }
        to { opacity: 1; }
      }

      @keyframes docsign-loading-fade-out {
        from { opacity: 1; }
        to { opacity: 0; }
      }

      .docsign-loading-content {
        background-color: var(--color-bg-primary, #ffffff);
        padding: 32px 48px;
        border-radius: 16px;
        text-align: center;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
        max-width: 90%;
      }

      .docsign-loading-spinner {
        color: var(--color-action-bg, #0056b3);
        margin-bottom: 16px;
      }

      .docsign-loading-spinner svg {
        display: block;
        margin: 0 auto;
      }

      .docsign-loading-message {
        font-size: var(--font-size-lg, 22px);
        color: var(--color-text-primary, #1a1a1a);
        margin: 0;
        font-weight: 500;
      }

      /* Reduced motion preference */
      @media (prefers-reduced-motion: reduce) {
        .docsign-loading-spinner svg circle {
          animation: none;
          stroke-dasharray: 45, 150;
        }
      }
    `,document.head.appendChild(e)}escapeHtml(e){let t=document.createElement("div");return t.textContent=e,t.innerHTML}},v=fe.getInstance();async function ot(i,e){v.start(i);try{return await e()}finally{v.end(i)}}function Wt(i,e){v.start(i);try{return e()}finally{v.end(i)}}async function st(i,e,t){let n=v.showLoading(i,t);try{return await e()}finally{v.hideLoading(n)}}typeof window<"u"&&(v.mark(y.BUNDLE_LOADED),window.DocSignPerf=v);var Z={debug:0,info:1,warn:2,error:3},m={minLevel:"info",enabled:!1,filter:null};function Kt(){if(typeof window>"u"){m.enabled=!1,m.minLevel="debug";return}try{let i=new URLSearchParams(window.location.search);if(i.has("log")){m.enabled=!0;let e=i.get("log");e&&e in Z&&(m.minLevel=e)}if(i.has("logFilter")&&(m.filter=i.get("logFilter")),!m.enabled&&typeof localStorage<"u"){let e=localStorage.getItem("docsign:log");(e==="1"||e==="true")&&(m.enabled=!0);let t=localStorage.getItem("docsign:logLevel");t&&t in Z&&(m.minLevel=t);let n=localStorage.getItem("docsign:logFilter");n&&(m.filter=n)}m.enabled}catch{}}Kt();function Jt(i){if(!m.filter)return!0;try{return new RegExp(m.filter,"i").test(i)}catch{return!0}}function g(i){let e=`[${i}]`,t=n=>!(!m.enabled||Z[n]<Z[m.minLevel]||!Jt(i));return{namespace:i,debug(n,...r){t("debug")},info(n,...r){t("info")},warn(n,...r){t("warn")},error(n,...r){t("error")}}}function Yt(i="info",e){m.enabled=!0,m.minLevel=i,e!==void 0&&(m.filter=e);try{typeof localStorage<"u"&&(localStorage.setItem("docsign:log","1"),localStorage.setItem("docsign:logLevel",i),e&&localStorage.setItem("docsign:logFilter",e))}catch{}}function Gt(){m.enabled=!1;try{typeof localStorage<"u"&&(localStorage.removeItem("docsign:log"),localStorage.removeItem("docsign:logLevel"),localStorage.removeItem("docsign:logFilter"))}catch{}}function Vt(){return{...m}}var Dn={DocSign:g("DocSign"),SyncManager:g("SyncManager"),LocalSessionManager:g("LocalSessionManager"),CryptoUtils:g("CryptoUtils"),PdfLoader:g("PdfLoader"),Perf:g("Perf")};typeof window<"u"&&(window.DocSignLog={enable:Yt,disable:Gt,config:Vt,create:g});var Qt=g("PdfLoader"),ve=!1,W=null;async function ee(){if(!ve)return W||(v.mark(y.PDFJS_LOAD_START),W=new Promise((i,e)=>{let t=document.createElement("script");t.src="./js/vendor/pdf.min.js",t.onload=()=>{window.pdfjsLib?(window.pdfjsLib.GlobalWorkerOptions.workerSrc="./js/vendor/pdf.worker.min.js",ve=!0,v.mark(y.PDFJS_LOADED),Qt.info("PDF.js loaded successfully (lazy)"),i()):e(new Error("PDF.js loaded but pdfjsLib not found on window"))},t.onerror=n=>{W=null;let r=n;e(new Error("Failed to load PDF.js: "+(r.message||"Unknown error")))},document.head.appendChild(t)}),W)}function Xt(){return ve}window.ensurePdfJsLoaded=ee;var k={currentDoc:null,pageCanvases:new Map,async loadDocument(i){await ee();let e=i instanceof Uint8Array?i:new Uint8Array(i);if(!window.pdfjsLib)throw new Error("PDF.js not loaded");return this.currentDoc=await window.pdfjsLib.getDocument(e).promise,this.currentDoc.numPages},async renderPage(i,e,t=1.5){if(!this.currentDoc)throw new Error("No document loaded");let n=await this.currentDoc.getPage(i),r=n.getViewport({scale:t});e.width=r.width,e.height=r.height;let o=e.getContext("2d");if(!o)throw new Error("Could not get 2d context");return await n.render({canvasContext:o,viewport:r}).promise,this.pageCanvases.set(i,{canvas:e,viewport:r,page:n}),{width:r.width,height:r.height,originalWidth:r.width/t,originalHeight:r.height/t,pdfWidth:n.view[2],pdfHeight:n.view[3]}},getPageDimensions(i){let e=this.pageCanvases.get(i);return e?{width:e.viewport.width,height:e.viewport.height}:null},getPageInfo(i){return this.pageCanvases.get(i)},async extractTextWithPositions(i){if(!this.currentDoc)throw new Error("No document loaded");let t=await(await this.currentDoc.getPage(i)).getTextContent(),r=this.pageCanvases.get(i)?.viewport,o=t.styles||{};return t.items.map((s,a)=>{let l=s.transform[4],u=s.transform[5],E=s.width||0,M=s.height||12,T=Math.abs(s.transform[3])||s.height||12,P=(s.fontName?o[s.fontName]:void 0)?.fontFamily||"sans-serif",me=(s.fontName||"").toLowerCase(),qt=me.includes("italic")||me.includes("oblique"),$t=me.includes("bold"),Ze=null,et=T;if(r){let[tt,nt]=r.convertToViewportPoint(l,u),[it,rt]=r.convertToViewportPoint(l+E,u+M);Ze={x:Math.min(tt,it),y:Math.min(nt,rt),width:Math.abs(it-tt)||E*r.scale,height:Math.abs(rt-nt)||M*r.scale},et=T*r.scale}return{index:a,str:s.str,pdfX:l,pdfY:u,pdfWidth:E,pdfHeight:M,fontSize:T,domFontSize:et,fontName:s.fontName,fontFamily:P,isItalic:qt,isBold:$t,domBounds:Ze}})},cleanup(){this.currentDoc&&(this.currentDoc.destroy(),this.currentDoc=null),this.pageCanvases.clear()}},Zt=k;window.PdfPreviewBridge=k;function en(i,e,t,n,r){let[o,s]=i.convertToPdfPoint(e,t),[a,l]=i.convertToPdfPoint(e+n,t+r);return{x:Math.min(o,a),y:Math.min(s,l),width:Math.abs(a-o),height:Math.abs(l-s)}}function tn(i,e,t){return i.convertToPdfPoint(e,t)}function nn(i,e,t,n,r){let o=[e,t,e+n,t+r],[s,a,l,u]=i.convertToViewportRectangle(o);return{x:Math.min(s,l),y:Math.min(a,u),width:Math.abs(l-s),height:Math.abs(u-a)}}function rn(i,e,t){return i.convertToViewportPoint(e,t)}function on(i,e){if(!i)return null;let t=e?.querySelector("canvas");return t?{canvas:t,canvasRect:t.getBoundingClientRect(),viewport:i.viewport}:null}var sn=[{category:"network",patterns:[/network/i,/fetch/i,/offline/i,/connection/i,/internet/i,/failed to load/i,/timeout/i,/ECONNREFUSED/i,/ENOTFOUND/i,/ERR_NETWORK/i,/net::/i]},{category:"password-protected",patterns:[/password/i,/encrypted/i,/protected/i,/decrypt/i,/locked/i,/access denied.*pdf/i]},{category:"signature-invalid",patterns:[/signature.*invalid/i,/invalid.*signature/i,/signature.*failed/i,/failed.*signature/i,/signature.*error/i,/sign.*failed/i,/could not sign/i,/signing.*error/i]},{category:"session-expired",patterns:[/session.*expired/i,/expired.*session/i,/link.*expired/i,/expired.*link/i,/session.*not found/i,/invalid.*session/i,/401/i,/403/i,/unauthorized/i]},{category:"file-corrupt",patterns:[/corrupt/i,/invalid.*pdf/i,/pdf.*invalid/i,/malformed/i,/cannot.*read/i,/parse.*error/i,/invalid.*document/i,/damaged/i]},{category:"authentication",patterns:[/authentication/i,/login/i,/credentials/i,/identity/i,/verification.*failed/i]}],an={network:{title:"Connection Problem",message:"We could not connect to the internet right now. Your document is completely safe and has not been lost. Please check your internet connection and try again when you are back online.",action:"Try Again",icon:"wifi-off"},"password-protected":{title:"This PDF is Password-Protected",message:"This document has a password that prevents us from opening it. Please contact the person who sent you this document and ask them for the password, or request an unprotected version.",action:"Enter Password",icon:"lock"},"signature-invalid":{title:"Signature Problem",message:"We had trouble adding your signature to the document. This sometimes happens if the signature was drawn too quickly. Please try drawing your signature again, taking your time with each stroke.",action:"Try Again",icon:"signature"},"session-expired":{title:"Signing Link Has Expired",message:"The link you used to sign this document is no longer active. This can happen if some time has passed since you received the email. Please contact the sender to request a new signing link.",action:"Request New Link",icon:"clock"},"file-corrupt":{title:"Document Problem",message:"We could not open this document because it may be damaged or in an unsupported format. Please contact the sender and ask them to send the document again.",action:"Contact Sender",icon:"file"},authentication:{title:"Identity Verification Problem",message:"We could not verify your identity to access this document. Please make sure you are using the correct signing link from your email.",action:"Check Link",icon:"user"},generic:{title:"Something Went Wrong",message:"We ran into an unexpected problem, but your document is safe. If this keeps happening, please contact the person who sent you this document for help.",action:"Go Back",icon:"alert"}};function te(i){let e=typeof i=="string"?i:i.message;for(let{patterns:t,category:n}of sn)for(let r of t)if(r.test(e))return n;return"generic"}function ye(i){let e=te(i);return{...an[e]}}function be(i,e,t,n="alert"){return{title:i,message:e,action:t,icon:n}}function we(){return{title:"You Are Offline",message:"Your device is not connected to the internet. Your work is saved locally and will be sent automatically when you reconnect. You can continue working offline.",action:"Continue",icon:"wifi-off"}}function Se(i=25){return{title:"File Is Too Large",message:`This file is larger than ${i} MB, which is the maximum size we can handle. Please contact the sender and ask them to send a smaller version of the document.`,action:"Go Back",icon:"file"}}function xe(){return{title:"Unsupported File Type",message:"We can only work with PDF documents. If you received a different type of file, please ask the sender to convert it to PDF format.",action:"Go Back",icon:"file"}}var at={"wifi-off":`<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <line x1="1" y1="1" x2="23" y2="23"></line>
    <path d="M16.72 11.06A10.94 10.94 0 0 1 19 12.55"></path>
    <path d="M5 12.55a10.94 10.94 0 0 1 5.17-2.39"></path>
    <path d="M10.71 5.05A16 16 0 0 1 22.58 9"></path>
    <path d="M1.42 9a15.91 15.91 0 0 1 4.7-2.88"></path>
    <path d="M8.53 16.11a6 6 0 0 1 6.95 0"></path>
    <line x1="12" y1="20" x2="12.01" y2="20"></line>
  </svg>`,lock:`<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
    <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
  </svg>`,signature:`<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M20 17.5a2.5 2.5 0 0 1-2.5 2.5H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h5.5L14 6.5H20a2 2 0 0 1 2 2v9"></path>
    <path d="M18 17l-3 3 1-6 5-5-3-3-5 5-6 1 3 3z"></path>
  </svg>`,clock:`<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <polyline points="12 6 12 12 16 14"></polyline>
  </svg>`,file:`<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path>
    <polyline points="13 2 13 9 20 9"></polyline>
  </svg>`,alert:`<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <line x1="12" y1="8" x2="12" y2="12"></line>
    <line x1="12" y1="16" x2="12.01" y2="16"></line>
  </svg>`,user:`<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
    <circle cx="12" cy="7" r="4"></circle>
  </svg>`},ln={error:`<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <line x1="15" y1="9" x2="9" y2="15"></line>
    <line x1="9" y1="9" x2="15" y2="15"></line>
  </svg>`,warning:`<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
    <line x1="12" y1="9" x2="12" y2="13"></line>
    <line x1="12" y1="17" x2="12.01" y2="17"></line>
  </svg>`,success:`<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
    <polyline points="22 4 12 14.01 9 11.01"></polyline>
  </svg>`},U=null,K=null,ne=null;function lt(){let i=document.createElement("div");return i.className="modal-overlay error-modal-overlay",i.setAttribute("role","dialog"),i.setAttribute("aria-modal","true"),i}function cn(i){let e=document.createElement("div");return e.className="modal-content error-modal-content confirm-dialog",e.setAttribute("role","alertdialog"),e.setAttribute("aria-labelledby","error-modal-title"),e.setAttribute("aria-describedby","error-modal-message"),e.innerHTML=`
    <div class="confirm-icon error-icon" aria-hidden="true">
      ${at[i.icon]}
    </div>
    <h2 id="error-modal-title" class="confirm-title error-title">${R(i.title)}</h2>
    <p id="error-modal-message" class="confirm-message error-modal-message">${R(i.message)}</p>
    <div class="confirm-actions error-modal-actions">
      <button type="button" class="btn-primary btn-large error-modal-action" data-action="primary">
        ${R(i.action)}
      </button>
      <button type="button" class="btn-secondary error-modal-dismiss" data-action="dismiss">
        Close
      </button>
    </div>
  `,e}function Ee(i,e,t){L();let n=lt(),r=cn(i);n.appendChild(r),U=n,document.body.appendChild(n);let o=r.querySelector('[data-action="primary"]'),s=r.querySelector('[data-action="dismiss"]'),a=r.querySelectorAll("button"),l=a[0],u=a[a.length-1];setTimeout(()=>{o?.focus()},100);let E=()=>{L(),e?.()},M=()=>{L(),t?.()},T=P=>{P.target===n&&(L(),t?.())},pe=P=>{P.key==="Escape"&&(L(),t?.()),P.key==="Tab"&&(P.shiftKey&&document.activeElement===l?(P.preventDefault(),u?.focus()):!P.shiftKey&&document.activeElement===u&&(P.preventDefault(),l?.focus()))};o?.addEventListener("click",E),s?.addEventListener("click",M),n.addEventListener("click",T),document.addEventListener("keydown",pe),n._cleanup=()=>{o?.removeEventListener("click",E),s?.removeEventListener("click",M),n.removeEventListener("click",T),document.removeEventListener("keydown",pe)}}function L(){if(U){let i=U._cleanup;i?.(),U.remove(),U=null}}function ke(i,e="error",t=5e3){H();let n=document.createElement("div");n.className=`error-toast error-toast-${e}`,n.setAttribute("role","alert"),n.setAttribute("aria-live",e==="error"?"assertive":"polite");let r=e==="error"?"alert-error":e==="warning"?"alert-warning":"alert-success";n.innerHTML=`
    <div class="error-toast-content ${r}">
      <span class="error-toast-icon" aria-hidden="true">
        ${ln[e]}
      </span>
      <span class="error-toast-message">${R(i)}</span>
      <button type="button" class="error-toast-close" aria-label="Close notification">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    </div>
  `,n.style.cssText=`
    position: fixed;
    bottom: var(--spacing-lg, 32px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 2000;
    max-width: calc(100% - var(--spacing-lg, 32px) * 2);
    width: 100%;
    max-width: 600px;
    animation: error-toast-slide-up 0.3s ease-out;
  `;let o=n.querySelector(".error-toast-content");o&&(o.style.cssText=`
      display: flex;
      align-items: center;
      gap: var(--spacing-sm, 16px);
      padding: var(--spacing-md, 24px);
      border-radius: var(--border-radius-lg, 12px);
      font-size: var(--font-size-lg, 22px);
      font-weight: 500;
      box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    `);let s=n.querySelector(".error-toast-close");if(s&&(s.style.cssText=`
      background: none;
      border: none;
      cursor: pointer;
      padding: 8px;
      margin-left: auto;
      opacity: 0.7;
      transition: opacity 0.2s;
      color: inherit;
      min-width: 44px;
      min-height: 44px;
      display: flex;
      align-items: center;
      justify-content: center;
    `,s.addEventListener("click",H),s.addEventListener("mouseenter",()=>{s.style.opacity="1"}),s.addEventListener("mouseleave",()=>{s.style.opacity="0.7"})),!document.getElementById("error-toast-styles")){let a=document.createElement("style");a.id="error-toast-styles",a.textContent=`
      @keyframes error-toast-slide-up {
        from {
          transform: translateX(-50%) translateY(100%);
          opacity: 0;
        }
        to {
          transform: translateX(-50%) translateY(0);
          opacity: 1;
        }
      }
      @keyframes error-toast-slide-down {
        from {
          transform: translateX(-50%) translateY(0);
          opacity: 1;
        }
        to {
          transform: translateX(-50%) translateY(100%);
          opacity: 0;
        }
      }
    `,document.head.appendChild(a)}document.body.appendChild(n),K=n,ne=setTimeout(()=>{H()},t)}function H(){if(ne&&(clearTimeout(ne),ne=null),K){K.style.animation="error-toast-slide-down 0.3s ease-in forwards";let i=K;K=null,setTimeout(()=>{i.remove()},300)}}function R(i){let e=document.createElement("div");return e.textContent=i,e.innerHTML}function Ce(i,e,t="Yes",n="No"){return new Promise(r=>{let o={title:i,message:e,action:t,icon:"alert"};L();let s=lt(),a=document.createElement("div");a.className="modal-content error-modal-content confirm-dialog",a.setAttribute("role","alertdialog"),a.setAttribute("aria-labelledby","confirm-modal-title"),a.setAttribute("aria-describedby","confirm-modal-message"),a.innerHTML=`
      <div class="confirm-icon" aria-hidden="true">
        ${at.alert}
      </div>
      <h2 id="confirm-modal-title" class="confirm-title">${R(i)}</h2>
      <p id="confirm-modal-message" class="confirm-message">${R(e)}</p>
      <div class="confirm-actions">
        <button type="button" class="btn-primary btn-large" data-action="confirm">
          ${R(t)}
        </button>
        <button type="button" class="btn-secondary" data-action="cancel">
          ${R(n)}
        </button>
      </div>
    `,s.appendChild(a),U=s,document.body.appendChild(s);let l=a.querySelector('[data-action="confirm"]'),u=a.querySelector('[data-action="cancel"]');setTimeout(()=>{l?.focus()},100);let E=()=>{L()};l?.addEventListener("click",()=>{E(),r(!0)}),u?.addEventListener("click",()=>{E(),r(!1)}),s.addEventListener("click",T=>{T.target===s&&(E(),r(!1))});let M=T=>{T.key==="Escape"&&(document.removeEventListener("keydown",M),E(),r(!1))};document.addEventListener("keydown",M)})}var ct="docsign_offline_queue";function dn(i){let e=JSON.parse(i);if(typeof e.sessionId!="string")throw new Error("Invalid sessionId in queued submission");if(typeof e.recipientId!="string")throw new Error("Invalid recipientId in queued submission");if(typeof e.signingKey!="string")throw new Error("Invalid signingKey in queued submission");if(typeof e.signatures!="object"||e.signatures===null)throw new Error("Invalid signatures in queued submission");if(typeof e.completedAt!="string")throw new Error("Invalid completedAt in queued submission");if(typeof e.timestamp!="number")throw new Error("Invalid timestamp in queued submission");return e}function J(){if(typeof localStorage>"u")return[];let i=localStorage.getItem(ct);if(!i)return[];try{let e=JSON.parse(i);return Array.isArray(e)?e.map(dn):[]}catch{return[]}}function Y(i,e){if(typeof localStorage>"u")return;let n=J().filter(r=>!(r.sessionId===i&&r.recipientId===e));localStorage.setItem(ct,JSON.stringify(n))}var f={STARTED:"docsign:sync-started",COMPLETED:"docsign:sync-completed",FAILED:"docsign:sync-failed",PROGRESS:"docsign:sync-progress",ONLINE_STATUS_CHANGED:"docsign:online-status-changed"};function dt(i){let e=new CustomEvent(f.STARTED,{detail:i,bubbles:!0});window.dispatchEvent(e)}function ut(i){let e=new CustomEvent(f.COMPLETED,{detail:i,bubbles:!0});window.dispatchEvent(e)}function ht(i){let e=new CustomEvent(f.FAILED,{detail:i,bubbles:!0});window.dispatchEvent(e)}function gt(i){let e=new CustomEvent(f.PROGRESS,{detail:i,bubbles:!0});window.dispatchEvent(e)}function Te(i){let e=new CustomEvent(f.ONLINE_STATUS_CHANGED,{detail:i,bubbles:!0});window.dispatchEvent(e)}function Pe(i){let e=t=>{i(t.detail)};return window.addEventListener(f.STARTED,e),()=>window.removeEventListener(f.STARTED,e)}function Le(i){let e=t=>{i(t.detail)};return window.addEventListener(f.COMPLETED,e),()=>window.removeEventListener(f.COMPLETED,e)}function Me(i){let e=t=>{i(t.detail)};return window.addEventListener(f.FAILED,e),()=>window.removeEventListener(f.FAILED,e)}function De(i){let e=t=>{i(t.detail)};return window.addEventListener(f.PROGRESS,e),()=>window.removeEventListener(f.PROGRESS,e)}function Re(i){let e=t=>{i(t.detail)};return window.addEventListener(f.ONLINE_STATUS_CHANGED,e),()=>window.removeEventListener(f.ONLINE_STATUS_CHANGED,e)}var h=g("SyncManager"),mt="docsign_sync_state",ft="docsign_sync_errors",Oe="docsign_offline_mode";function un(){if(typeof localStorage>"u")return{lastSyncAttempt:null,lastSuccessfulSync:null};try{let i=localStorage.getItem(mt);if(i)return JSON.parse(i)}catch{}return{lastSyncAttempt:null,lastSuccessfulSync:null}}function pt(i){typeof localStorage>"u"||localStorage.setItem(mt,JSON.stringify(i))}function hn(){if(typeof localStorage>"u")return[];try{let i=localStorage.getItem(ft);if(i)return JSON.parse(i)}catch{}return[]}function Ae(i){typeof localStorage>"u"||localStorage.setItem(ft,JSON.stringify(i))}function _(){return typeof localStorage>"u"?!1:localStorage.getItem(Oe)==="true"}var A=class{constructor(e){this.isSyncing=!1;this.isStarted=!1;this.retryTimeoutId=null;this.periodicRetryId=null;this.handleOnline=()=>{h.info("Device came online"),Te({online:!0,timestamp:new Date().toISOString()}),_()||setTimeout(()=>this.syncNow(),1e3)};this.handleOffline=()=>{h.info("Device went offline"),Te({online:!1,timestamp:new Date().toISOString()})};this.config={syncEndpoint:e.syncEndpoint,minBackoffMs:e.minBackoffMs??1e3,maxBackoffMs:e.maxBackoffMs??3e4,retryIntervalMs:e.retryIntervalMs??3e4,maxRetries:e.maxRetries??10},this.persistedState=un(),this.errors=hn()}start(){if(this.isStarted){h.debug("Already started");return}this.isStarted=!0,h.info("Starting sync manager"),window.addEventListener("online",this.handleOnline),window.addEventListener("offline",this.handleOffline),navigator.onLine&&!_()&&this.syncNow(),this.startPeriodicRetry()}stop(){this.isStarted&&(this.isStarted=!1,h.info("Stopping sync manager"),window.removeEventListener("online",this.handleOnline),window.removeEventListener("offline",this.handleOffline),this.retryTimeoutId&&(clearTimeout(this.retryTimeoutId),this.retryTimeoutId=null),this.periodicRetryId&&(clearInterval(this.periodicRetryId),this.periodicRetryId=null))}async syncNow(){if(_()){h.debug("Skipping sync - explicit offline mode");return}if(!navigator.onLine){h.debug("Skipping sync - offline");return}if(this.isSyncing){h.debug("Skipping sync - already in progress");return}let e=J();if(e.length===0){h.debug("Nothing to sync");return}this.isSyncing=!0;let t=Date.now(),n=new Date().toISOString();this.persistedState.lastSyncAttempt=n,pt(this.persistedState),dt({pendingCount:e.length,timestamp:n}),h.info(`Starting sync of ${e.length} items`);let r=0;for(let a=0;a<e.length;a++){let l=e[a];gt({current:a+1,total:e.length,sessionId:l.sessionId,percentage:Math.round((a+1)/e.length*100)}),await this.syncItem(l)&&r++}this.isSyncing=!1;let o=new Date().toISOString(),s=Date.now()-t;r===e.length&&(this.persistedState.lastSuccessfulSync=o,pt(this.persistedState)),ut({syncedCount:r,timestamp:o,durationMs:s}),h.info(`Sync completed: ${r}/${e.length} items`)}getStatus(){return{pendingCount:J().length,lastSyncAttempt:this.persistedState.lastSyncAttempt,lastSuccessfulSync:this.persistedState.lastSuccessfulSync,isSyncing:this.isSyncing,isOnline:navigator.onLine,errors:[...this.errors]}}clearErrors(){this.errors=[],Ae(this.errors),h.debug("Errors cleared")}setOfflineMode(e){typeof localStorage>"u"||(e?(localStorage.setItem(Oe,"true"),h.info("Offline mode enabled")):(localStorage.removeItem(Oe),h.info("Offline mode disabled"),navigator.onLine&&this.syncNow()))}isOfflineModeEnabled(){return _()}notifyNewSignature(){h.debug("New signature saved, checking for sync"),navigator.onLine&&!_()&&!this.isSyncing&&setTimeout(()=>this.syncNow(),500)}startPeriodicRetry(){this.periodicRetryId||(this.periodicRetryId=setInterval(()=>{navigator.onLine&&!_()&&!this.isSyncing&&J().length>0&&(h.debug("Periodic retry triggered"),this.syncNow())},this.config.retryIntervalMs))}async syncItem(e){let t=`${e.sessionId}:${e.recipientId}`,r=(this.errors.find(o=>o.sessionId===e.sessionId&&o.recipientId===e.recipientId)?.attemptCount??0)+1;if(r>this.config.maxRetries)return h.warn(`Max retries exceeded for ${t}, skipping`),!1;try{let o=await this.postSignature(e);if(o.ok)return Y(e.sessionId,e.recipientId),this.removeError(e.sessionId,e.recipientId),h.debug(`Successfully synced ${t}`),!0;if(o.status===409){let a=await o.json();return await this.handleConflict(e,a),!0}let s=await o.text();return this.recordError(e,`Server error ${o.status}: ${s}`,r),this.scheduleRetry(e,r),!1}catch(o){let s=o instanceof Error?o.message:String(o);return h.error(`Failed to sync ${t}:`,s),this.recordError(e,s,r),this.scheduleRetry(e,r),!1}}async postSignature(e){return fetch(this.config.syncEndpoint,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({sessionId:e.sessionId,recipientId:e.recipientId,signingKey:e.signingKey,signatures:e.signatures,completedAt:e.completedAt,clientTimestamp:e.timestamp})})}async handleConflict(e,t){if(h.debug(`Handling conflict for ${e.sessionId}`),t.serverTimestamp&&t.serverTimestamp>e.timestamp){h.debug("Server has newer data, preferring server"),Y(e.sessionId,e.recipientId);return}if(t.signatures&&e.signatures){h.debug("Merging local signatures with server");let n={...t.signatures,...e.signatures},r={...e,signatures:n};(await this.postSignature(r)).ok&&(Y(e.sessionId,e.recipientId),h.debug("Conflict resolved with merge"))}else Y(e.sessionId,e.recipientId);this.removeError(e.sessionId,e.recipientId)}recordError(e,t,n){let r=new Date().toISOString();this.removeError(e.sessionId,e.recipientId);let o={sessionId:e.sessionId,recipientId:e.recipientId,error:t,attemptCount:n,lastAttempt:r};this.errors.push(o),Ae(this.errors),ht({sessionId:e.sessionId,error:t,attemptCount:n,timestamp:r,willRetry:n<this.config.maxRetries})}removeError(e,t){let n=this.errors.findIndex(r=>r.sessionId===e&&r.recipientId===t);n!==-1&&(this.errors.splice(n,1),Ae(this.errors))}scheduleRetry(e,t){if(t>=this.config.maxRetries)return;let n=Math.min(this.config.minBackoffMs*Math.pow(2,t-1),this.config.maxBackoffMs);h.debug(`Scheduling retry for ${e.sessionId} in ${n}ms (attempt ${t})`)}},Ie=null;function ie(i){if(!Ie){if(!i)throw new Error("SyncManager not initialized. Call getSyncManager with config first.");Ie=new A(i)}return Ie}function re(i){let e=ie(i);return e.start(),e}var I=g("DocSign");function gn(i){let e=i.replace(/^data:[^;]+;base64,/,""),t=atob(e),n=new Uint8Array(t.length);for(let r=0;r<t.length;r++)n[r]=t.charCodeAt(r);return n}function pn(){let i=0;return{async loadPdf(e){try{k.cleanup();let t;return typeof e=="string"?t=gn(e):t=e,i=await k.loadDocument(t),I.info("PDF loaded:",i,"pages"),{numPages:i,success:!0}}catch(t){let n=t instanceof Error?t.message:String(t);return I.error("Failed to load PDF:",n),i=0,{numPages:0,success:!1,error:n}}},async renderAllPages(e){let{container:t,scale:n=1.5,pageWrapperClass:r="pdf-page-wrapper"}=e,o=[];if(!k.currentDoc)return I.error("No document loaded"),o;t.innerHTML="";for(let s=1;s<=i;s++){let a=document.createElement("div");a.className=r,a.dataset.pageNumber=String(s);let l=document.createElement("canvas");a.appendChild(l),t.appendChild(a);let u=await this.renderPage(s,l,n);o.push(u)}return I.debug("Rendered",o.length,"pages"),o},async renderPage(e,t,n=1.5){try{let r=await k.renderPage(e,t,n);return{pageNum:e,dimensions:r,canvas:t,success:!0}}catch(r){let o=r instanceof Error?r.message:String(r);return I.error(`Failed to render page ${e}:`,o),{pageNum:e,dimensions:{width:0,height:0,originalWidth:0,originalHeight:0,pdfWidth:0,pdfHeight:0},canvas:t,success:!1,error:o}}},getPageCount(){return i},getPageDimensions(e){return k.getPageDimensions(e)},cleanup(){k.cleanup(),i=0,I.debug("Cleaned up PDF resources")},isDocumentLoaded(){return k.currentDoc!==null}}}var w=pn();function vt(){window.DocSign={loadPdf:w.loadPdf.bind(w),renderAllPages:w.renderAllPages.bind(w),renderPage:w.renderPage.bind(w),getPageCount:w.getPageCount.bind(w),getPageDimensions:w.getPageDimensions.bind(w),cleanup:w.cleanup.bind(w),isDocumentLoaded:w.isDocumentLoaded.bind(w),getUserFriendlyError:ye,categorizeError:te,createUserError:be,getOfflineError:we,getFileTooLargeError:Se,getUnsupportedFileError:xe,showErrorModal:Ee,hideErrorModal:L,showErrorToast:ke,hideErrorToast:H,showConfirmDialog:Ce,SyncManager:A,getSyncManager:ie,initSyncManager:re,SYNC_EVENTS:f,onSyncStarted:Pe,onSyncCompleted:Le,onSyncFailed:Me,onSyncProgress:De,onOnlineStatusChanged:Re},I.info("PDF bridge, error handling, and sync manager initialized on window.DocSign")}var bt=g("CryptoUtils"),yt="docsign_crypto_seed",G="AES-GCM",mn=256,wt=12,oe=null;function fn(){if(typeof localStorage>"u")throw new Error("localStorage is not available");let i=localStorage.getItem(yt);if(!i){let e=new Uint8Array(32);crypto.getRandomValues(e),i=Array.from(e).map(t=>t.toString(16).padStart(2,"0")).join(""),localStorage.setItem(yt,i),bt.debug("Generated new device seed")}return i}async function se(){if(oe)return oe;let i=fn(),e=new Uint8Array(i.match(/.{2}/g).map(r=>parseInt(r,16))),t=await crypto.subtle.importKey("raw",e,"PBKDF2",!1,["deriveKey"]),n=new TextEncoder().encode("docsign-indexeddb-encryption-v1");return oe=await crypto.subtle.deriveKey({name:"PBKDF2",salt:n,iterations:1e5,hash:"SHA-256"},t,{name:G,length:mn},!1,["encrypt","decrypt"]),bt.debug("Derived encryption key"),oe}async function St(i){let e=await se(),t=crypto.getRandomValues(new Uint8Array(wt)),n=new TextEncoder().encode(i),r=await crypto.subtle.encrypt({name:G,iv:t},e,n),o=btoa(String.fromCharCode(...new Uint8Array(r))),s=btoa(String.fromCharCode(...t));return{ciphertext:o,iv:s,version:1}}async function xt(i){let e=await se(),t=Uint8Array.from(atob(i.ciphertext),o=>o.charCodeAt(0)),n=Uint8Array.from(atob(i.iv),o=>o.charCodeAt(0)),r=await crypto.subtle.decrypt({name:G,iv:n},e,t);return new TextDecoder().decode(r)}async function Et(i){let e=await se(),t=crypto.getRandomValues(new Uint8Array(wt)),n=new ArrayBuffer(i.length);new Uint8Array(n).set(i);let r=await crypto.subtle.encrypt({name:G,iv:t},e,n),o=btoa(String.fromCharCode(...new Uint8Array(r))),s=btoa(String.fromCharCode(...t));return{ciphertext:o,iv:s,version:1}}async function kt(i){let e=await se(),t=Uint8Array.from(atob(i.ciphertext),a=>a.charCodeAt(0)),n=Uint8Array.from(atob(i.iv),a=>a.charCodeAt(0)),r=new ArrayBuffer(t.length);new Uint8Array(r).set(t);let o=new ArrayBuffer(n.length);new Uint8Array(o).set(n);let s=await crypto.subtle.decrypt({name:G,iv:new Uint8Array(o)},e,r);return new Uint8Array(s)}function Fe(i){if(!i||typeof i!="object")return!1;let e=i;return typeof e.ciphertext=="string"&&typeof e.iv=="string"&&e.version===1}function Ne(){return typeof crypto<"u"&&typeof crypto.subtle<"u"&&typeof localStorage<"u"}var c=g("LocalSessionManager"),vn="docsign_local",yn=1,p={SESSIONS:"sessions",PDF_CACHE:"pdf_cache",SIGNATURE_QUEUE:"signature_queue"};function V(){return new Promise((i,e)=>{let t=indexedDB.open(vn,yn);t.onerror=()=>{c.error("Failed to open database:",t.error),e(t.error)},t.onsuccess=()=>{i(t.result)},t.onupgradeneeded=n=>{let r=n.target.result;if(!r.objectStoreNames.contains(p.SESSIONS)){let o=r.createObjectStore(p.SESSIONS,{keyPath:"sessionId"});o.createIndex("recipientId","recipientId",{unique:!1}),o.createIndex("status","status",{unique:!1}),o.createIndex("createdAt","createdAt",{unique:!1})}r.objectStoreNames.contains(p.PDF_CACHE)||r.createObjectStore(p.PDF_CACHE,{keyPath:"sessionId"}),r.objectStoreNames.contains(p.SIGNATURE_QUEUE)||r.createObjectStore(p.SIGNATURE_QUEUE,{keyPath:["sessionId","recipientId"]}).createIndex("timestamp","timestamp",{unique:!1}),c.info("Database schema created/upgraded")}})}async function Ct(i,e){let t=await V();return new Promise((n,r)=>{let a=t.transaction(i,"readonly").objectStore(i).get(e);a.onsuccess=()=>{t.close(),n(a.result)},a.onerror=()=>{t.close(),r(a.error)}})}async function Be(i,e){let t=await V();return new Promise((n,r)=>{let a=t.transaction(i,"readwrite").objectStore(i).put(e);a.onsuccess=()=>{t.close(),n()},a.onerror=()=>{t.close(),r(a.error)}})}async function Ue(i,e){let t=await V();return new Promise((n,r)=>{let a=t.transaction(i,"readwrite").objectStore(i).delete(e);a.onsuccess=()=>{t.close(),n()},a.onerror=()=>{t.close(),r(a.error)}})}async function He(i){let e=await V();return new Promise((t,n)=>{let s=e.transaction(i,"readonly").objectStore(i).getAll();s.onsuccess=()=>{e.close(),t(s.result)},s.onerror=()=>{e.close(),n(s.error)}})}var S=class{static async getSession(e,t){try{let n=await Ct(p.SESSIONS,e);if(!n){c.debug("Session not found locally:",e);return}if(n.expiresAt){let r=new Date(n.expiresAt).getTime();if(Date.now()>r)return c.debug("Session expired:",e),n.status="expired",n}return c.debug("Session found locally:",e),n}catch(n){c.error("Error getting session:",n);return}}static async saveSession(e){try{await Be(p.SESSIONS,e),c.debug("Session saved:",e.sessionId)}catch(t){throw c.error("Error saving session:",t),t}}static async cacheSession(e){try{let t={sessionId:String(e.sessionId||e.session_id||""),recipientId:String(e.recipientId||e.recipient_id||""),documentName:String(e.documentName||e.document_name||"Document"),metadata:e.metadata,fields:e.fields||[],recipients:e.recipients,status:e.status||"pending",createdAt:String(e.createdAt||e.created_at||new Date().toISOString()),expiresAt:e.expiresAt||e.expires_at,isServerCached:!0,lastSyncedAt:new Date().toISOString()};await this.saveSession(t),c.debug("Server session cached:",t.sessionId)}catch(t){c.error("Error caching server session:",t)}}static async cachePdfData(e,t){try{let n;if(typeof t=="string"){let s=atob(t);n=new Uint8Array(s.length);for(let a=0;a<s.length;a++)n[a]=s.charCodeAt(a)}else n=t;let r,o=!1;if(Ne())try{r=await Et(n),o=!0,c.debug("PDF data encrypted")}catch(s){c.warn("Encryption failed, storing unencrypted:",s),r=btoa(String.fromCharCode(...n))}else r=btoa(String.fromCharCode(...n));await Be(p.PDF_CACHE,{sessionId:e,pdfData:r,isEncrypted:o,cachedAt:new Date().toISOString()}),c.debug("PDF data cached for session:",e)}catch(n){c.error("Error caching PDF data:",n)}}static async getCachedPdfData(e){try{let t=await Ct(p.PDF_CACHE,e);if(!t)return;if(t.isEncrypted&&Fe(t.pdfData))try{let n=await kt(t.pdfData);return btoa(String.fromCharCode(...n))}catch(n){c.error("Failed to decrypt PDF data:",n);return}return t.pdfData}catch(t){c.error("Error getting cached PDF:",t);return}}static async saveSignatures(e,t){try{let n=await this.getSession(e);if(n){let r,o=!1,s={...n.signatures,...t};if(Ne())try{let a=JSON.stringify(s);r=await St(a),o=!0,c.debug("Signatures encrypted")}catch(a){c.warn("Signature encryption failed:",a),r=s}else r=s;n.signatures=r,n.status="in_progress",n.metadata||(n.metadata={}),n.metadata.signaturesEncrypted=o,await this.saveSession(n),c.debug("Signatures saved for session:",e)}else c.warn("Cannot save signatures - session not found:",e)}catch(n){throw c.error("Error saving signatures:",n),n}}static async getDecryptedSignatures(e){try{let t=await this.getSession(e);if(!t?.signatures)return;if(t.metadata?.signaturesEncrypted&&Fe(t.signatures))try{let r=await xt(t.signatures);return JSON.parse(r)}catch(r){c.error("Failed to decrypt signatures:",r);return}return t.signatures}catch(t){c.error("Error getting signatures:",t);return}}static async completeSession(e){try{let t=await this.getSession(e);t&&(t.status="completed",await this.saveSession(t),c.debug("Session completed:",e))}catch(t){throw c.error("Error completing session:",t),t}}static async queueForSync(e){try{await Be(p.SIGNATURE_QUEUE,e),c.debug("Submission queued for sync:",e.sessionId)}catch(t){throw c.error("Error queueing submission:",t),t}}static async getQueuedSubmissions(){try{return await He(p.SIGNATURE_QUEUE)}catch(e){return c.error("Error getting queued submissions:",e),[]}}static async removeFromQueue(e,t){try{await Ue(p.SIGNATURE_QUEUE,[e,t]),c.debug("Removed from queue:",e,t)}catch(n){c.error("Error removing from queue:",n)}}static async deleteSession(e){try{await Ue(p.SESSIONS,e),await Ue(p.PDF_CACHE,e),c.debug("Session deleted:",e)}catch(t){c.error("Error deleting session:",t)}}static async getSessionsForRecipient(e){try{return(await He(p.SESSIONS)).filter(n=>n.recipientId===e)}catch(t){return c.error("Error getting sessions for recipient:",t),[]}}static async clearAll(){try{let e=await V();await new Promise((t,n)=>{let r=e.transaction([p.SESSIONS,p.PDF_CACHE,p.SIGNATURE_QUEUE],"readwrite");r.objectStore(p.SESSIONS).clear(),r.objectStore(p.PDF_CACHE).clear(),r.objectStore(p.SIGNATURE_QUEUE).clear(),r.oncomplete=()=>{e.close(),t()},r.onerror=()=>{e.close(),n(r.error)}}),c.info("All local data cleared")}catch(e){c.error("Error clearing all data:",e)}}static isAvailable(){return typeof indexedDB<"u"}};var _e=class{async createSession(e,t,n=[]){let r=crypto.randomUUID(),o=new Date().toISOString(),s={sessionId:r,recipientId:t[0]?.id?.toString()||"",documentName:"Untitled Document",fields:n,recipients:t,status:"pending",createdAt:o,expiresAt:null};return e.length>0&&await S.cachePdfData(r,e),await S.saveSession(s),{id:r,recipients:t,fields:n,status:"pending",createdAt:o,expiresAt:null}}async getSession(e){let t=await S.getSession(e);return t?{id:t.sessionId,recipients:t.recipients||[],fields:t.fields,status:t.status,createdAt:t.createdAt,expiresAt:t.expiresAt||null}:null}async updateSessionStatus(e,t){let n=await S.getSession(e);n&&(n.status=t,await S.saveSession(n))}async recordSignature(e,t,n,r="draw",o=""){let s=await S.getSession(e);if(s){let a=s.signatures||{};a[t]={fieldId:t,type:r,data:n,timestamp:new Date().toISOString(),recipientId:o},await S.saveSignatures(e,a)}}async getSignedDocument(e){let t=await S.getCachedPdfData(e);if(!t)return null;let n=atob(t),r=new Uint8Array(n.length);for(let o=0;o<n.length;o++)r[o]=n.charCodeAt(o);return r}async deleteSession(e){await S.deleteSession(e)}async listSessions(){return(await He(p.SESSIONS)).map(t=>({id:t.sessionId,recipientCount:t.recipients?.length||0,fieldCount:t.fields.length,completedFieldCount:Object.keys(t.signatures||{}).length,status:t.status,createdAt:t.createdAt,expiresAt:t.expiresAt||null}))}},b=new _e;function Tt(){window.DocSign||(window.DocSign={});let i=window.DocSign;i.LocalSessionManager=S,i.localSessionManager=b,i.createSession=b.createSession.bind(b),i.getSession=b.getSession.bind(b),i.updateSessionStatus=b.updateSessionStatus.bind(b),i.recordSignature=b.recordSignature.bind(b),i.getSignedDocument=b.getSignedDocument.bind(b),i.deleteSession=b.deleteSession.bind(b),i.listSessions=b.listSessions.bind(b),c.info("Session management initialized on window.DocSign")}var O=[{name:"Dancing Script",label:"Classic Cursive",style:"flowing"},{name:"Great Vibes",label:"Elegant Script",style:"formal"},{name:"Pacifico",label:"Casual Handwriting",style:"casual"},{name:"Sacramento",label:"Flowing Script",style:"flowing"},{name:"Allura",label:"Formal Calligraphy",style:"calligraphy"}],D=class{constructor(e){this.inputElement=null;this.fontSelectorContainer=null;this.previewCanvas=null;this.previewContainer=null;this.text="";this.destroyed=!1;this.container=e.container,this.fonts=e.fonts||O.map(t=>t.name),this.currentFont=e.defaultFont||"Dancing Script",this.fontSize=e.fontSize||48,this.textColor=e.textColor||"#000080",this.backgroundColor=e.backgroundColor||"#ffffff",this.placeholder=e.placeholder||"Type your name",this.onChange=e.onChange,this.render()}setText(e){this.text=e,this.inputElement&&this.inputElement.value!==e&&(this.inputElement.value=e),this.updatePreview(),this.onChange?.(this.text,this.currentFont)}getText(){return this.text}setFont(e){this.fonts.includes(e)&&(this.currentFont=e,this.updateFontSelection(),this.updatePreview(),this.onChange?.(this.text,this.currentFont))}getFont(){return this.currentFont}isEmpty(){return!this.text||this.text.trim()===""}toDataURL(){return this.toCanvas().toDataURL("image/png")}toCanvas(){let e=window.devicePixelRatio||1,t=400,n=100,r=document.createElement("canvas");r.width=t*e,r.height=n*e;let o=r.getContext("2d");if(!o)throw new Error("Could not get canvas 2D context");if(o.scale(e,e),o.fillStyle=this.backgroundColor,o.fillRect(0,0,t,n),this.text.trim()){o.font=`${this.fontSize}px '${this.currentFont}', cursive`,o.fillStyle=this.textColor,o.textAlign="center",o.textBaseline="middle";let s=o.measureText(this.text),a=s.actualBoundingBoxAscent+s.actualBoundingBoxDescent,l=(n-a)/2+s.actualBoundingBoxAscent;o.fillText(this.text,t/2,l||n/2,t-20)}return r}destroy(){this.destroyed=!0,this.container.innerHTML="",this.inputElement=null,this.fontSelectorContainer=null,this.previewCanvas=null,this.previewContainer=null}render(){this.container.innerHTML="";let e=document.createElement("div");e.className="typed-signature-wrapper",e.style.cssText=`
      display: flex;
      flex-direction: column;
      gap: 1.5rem;
    `,this.inputElement=this.createInput(),e.appendChild(this.inputElement);let t=document.createElement("div");t.className="typed-signature-font-section";let n=document.createElement("label");n.textContent="Choose a style for your signature",n.style.cssText=`
      display: block;
      font-size: 18px;
      font-weight: 600;
      color: #111827;
      margin-bottom: 1rem;
    `,t.appendChild(n),this.fontSelectorContainer=this.createFontSelector(),t.appendChild(this.fontSelectorContainer),e.appendChild(t),this.previewContainer=this.createPreviewSection(),e.appendChild(this.previewContainer),this.container.appendChild(e),this.updatePreview()}createInput(){let e=document.createElement("input");return e.type="text",e.placeholder=this.placeholder,e.className="typed-signature-input",e.autocomplete="name",e.autocapitalize="words",e.style.cssText=`
      width: 100%;
      min-height: 60px;
      padding: 16px 20px;
      border: 2px solid #d1d5db;
      border-radius: 8px;
      font-size: 24px;
      font-family: inherit;
      text-align: center;
      color: #111827;
      background: #ffffff;
      transition: border-color 0.2s, box-shadow 0.2s;
      outline: none;
    `,e.addEventListener("focus",()=>{e.style.borderColor="#1e40af",e.style.boxShadow="0 0 0 3px rgba(30, 64, 175, 0.1)"}),e.addEventListener("blur",()=>{e.style.borderColor="#d1d5db",e.style.boxShadow="none"}),e.addEventListener("input",()=>{this.text=e.value,this.updatePreview(),this.updateFontPreviews(),this.onChange?.(this.text,this.currentFont)}),e}createFontSelector(){let e=document.createElement("div");return e.className="typed-signature-fonts",e.setAttribute("role","radiogroup"),e.setAttribute("aria-label","Signature style"),e.style.cssText=`
      display: flex;
      flex-direction: column;
      gap: 12px;
    `,this.fonts.forEach((t,n)=>{let r=this.createFontOption(t,n);e.appendChild(r)}),e}createFontOption(e,t){let r=O.find(E=>E.name===e)?.label||e,o=document.createElement("label");o.className="typed-signature-font-option",o.style.cssText=`
      display: flex;
      align-items: center;
      gap: 16px;
      padding: 16px;
      border: 2px solid ${this.currentFont===e?"#1e40af":"#e5e7eb"};
      border-radius: 12px;
      cursor: pointer;
      background: ${this.currentFont===e?"rgba(30, 64, 175, 0.05)":"#ffffff"};
      transition: all 0.2s;
    `,o.addEventListener("mouseenter",()=>{this.currentFont!==e&&(o.style.borderColor="#9ca3af",o.style.background="#f9fafb")}),o.addEventListener("mouseleave",()=>{this.currentFont!==e&&(o.style.borderColor="#e5e7eb",o.style.background="#ffffff")});let s=document.createElement("input");s.type="radio",s.name="signature-font",s.value=e,s.checked=this.currentFont===e,s.id=`font-${t}`,s.style.cssText=`
      width: 32px;
      height: 32px;
      margin: 0;
      cursor: pointer;
      accent-color: #1e40af;
      flex-shrink: 0;
    `,s.addEventListener("change",()=>{s.checked&&this.setFont(e)});let a=document.createElement("div");a.className="font-preview-area",a.style.cssText=`
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 4px;
    `;let l=document.createElement("span");l.textContent=r,l.style.cssText=`
      font-size: 14px;
      color: #6b7280;
      font-weight: 500;
    `;let u=document.createElement("span");return u.className="font-preview-text",u.dataset.font=e,u.textContent=this.text||"Your Name",u.style.cssText=`
      font-family: '${e}', cursive;
      font-size: 32px;
      color: ${this.textColor};
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    `,a.appendChild(l),a.appendChild(u),o.appendChild(s),o.appendChild(a),o}createPreviewSection(){let e=document.createElement("div");e.className="typed-signature-preview-section",e.style.cssText=`
      margin-top: 0.5rem;
    `;let t=document.createElement("label");t.textContent="Your signature will look like:",t.style.cssText=`
      display: block;
      font-size: 16px;
      color: #6b7280;
      margin-bottom: 0.75rem;
    `,e.appendChild(t);let n=document.createElement("div");return n.style.cssText=`
      background: #ffffff;
      border: 2px solid #e5e7eb;
      border-radius: 12px;
      padding: 1.5rem;
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100px;
    `,this.previewCanvas=document.createElement("canvas"),this.previewCanvas.width=400,this.previewCanvas.height=100,this.previewCanvas.style.cssText=`
      max-width: 100%;
      height: auto;
      display: block;
    `,n.appendChild(this.previewCanvas),e.appendChild(n),e}updatePreview(){if(this.destroyed||!this.previewCanvas)return;let e=this.previewCanvas.getContext("2d");if(!e)return;let t=this.previewCanvas.width,n=this.previewCanvas.height;e.fillStyle=this.backgroundColor,e.fillRect(0,0,t,n),this.text.trim()?(e.font=`${this.fontSize}px '${this.currentFont}', cursive`,e.fillStyle=this.textColor,e.textAlign="center",e.textBaseline="middle",e.fillText(this.text,t/2,n/2,t-20)):(e.font="16px system-ui, sans-serif",e.fillStyle="#9ca3af",e.textAlign="center",e.textBaseline="middle",e.fillText("Type your name above to see preview",t/2,n/2))}updateFontPreviews(){if(this.destroyed||!this.fontSelectorContainer)return;this.fontSelectorContainer.querySelectorAll(".font-preview-text").forEach(t=>{t.textContent=this.text||"Your Name"})}updateFontSelection(){if(this.destroyed||!this.fontSelectorContainer)return;this.fontSelectorContainer.querySelectorAll(".typed-signature-font-option").forEach(t=>{let n=t.querySelector("input[type='radio']"),r=n?.value===this.currentFont;n&&(n.checked=r),t.style.borderColor=r?"#1e40af":"#e5e7eb",t.style.background=r?"rgba(30, 64, 175, 0.05)":"#ffffff"})}};function je(i){return new D(i)}typeof window<"u"&&(window.TypedSignature=D,window.createTypedSignature=je,window.SIGNATURE_FONTS=O);var F=class{constructor(e={}){this.modalElement=null;this.canvasElement=null;this.ctx=null;this.isModalOpen=!1;this.isDrawing=!1;this.points=[];this.strokes=[];this.currentStroke=null;this.animationFrameId=null;this.activeTouchId=null;this.penColor="#000000";this.penWidth=3;this.resizeTimeout=null;this.focusableElements=[];this.firstFocusable=null;this.lastFocusable=null;this.previousActiveElement=null;this.resolvePromise=null;this.options={title:e.title??"Sign Here",instructions:e.instructions??"Draw your signature with your finger",onComplete:e.onComplete??(()=>{}),onCancel:e.onCancel??(()=>{})},this.handleKeyDown=this.handleKeyDown.bind(this),this.handleResize=this.handleResize.bind(this),this.handleOrientationChange=this.handleOrientationChange.bind(this),this.handleTouchStart=this.handleTouchStart.bind(this),this.handleTouchMove=this.handleTouchMove.bind(this),this.handleTouchEnd=this.handleTouchEnd.bind(this),this.handleMouseDown=this.handleMouseDown.bind(this),this.handleMouseMove=this.handleMouseMove.bind(this),this.handleMouseUp=this.handleMouseUp.bind(this)}open(){return new Promise(e=>{this.resolvePromise=e,this.showModal()})}close(){this.hideModal(null)}isOpen(){return this.isModalOpen}showModal(){this.isModalOpen||(this.previousActiveElement=document.activeElement,this.createModalDOM(),document.body.appendChild(this.modalElement),document.body.style.overflow="hidden",document.body.style.position="fixed",document.body.style.width="100%",document.body.style.height="100%",this.addEventListeners(),requestAnimationFrame(()=>{this.initializeCanvas(),this.setupFocusTrap(),this.modalElement.offsetHeight,this.modalElement.classList.add("mobile-signature-modal--visible")}),this.isModalOpen=!0)}hideModal(e){this.isModalOpen&&(this.removeEventListeners(),this.modalElement?.classList.remove("mobile-signature-modal--visible"),setTimeout(()=>{document.body.style.overflow="",document.body.style.position="",document.body.style.width="",document.body.style.height="",this.modalElement?.remove(),this.modalElement=null,this.canvasElement=null,this.ctx=null,this.previousActiveElement&&"focus"in this.previousActiveElement&&this.previousActiveElement.focus(),this.isModalOpen=!1,this.resolvePromise&&(this.resolvePromise(e),this.resolvePromise=null),e?this.options.onComplete(e.dataUrl):this.options.onCancel()},200))}createModalDOM(){this.injectStyles();let e=document.createElement("div");e.className="mobile-signature-modal",e.setAttribute("role","dialog"),e.setAttribute("aria-modal","true"),e.setAttribute("aria-label",this.options.title);let t=this.isPortrait();e.innerHTML=`
      <div class="mobile-signature-header">
        <h2 class="mobile-signature-title">${this.escapeHtml(this.options.title)}</h2>
        <button
          type="button"
          class="mobile-signature-close"
          aria-label="Close"
          data-action="close"
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      <div class="mobile-signature-canvas-area">
        <p class="mobile-signature-instructions">${this.escapeHtml(this.options.instructions)}</p>
        ${t?this.createRotateHint():""}
        <div class="mobile-signature-canvas-wrapper">
          <canvas
            class="mobile-signature-canvas"
            aria-label="Signature drawing area"
            tabindex="0"
          ></canvas>
          <div class="mobile-signature-baseline">Sign above this line</div>
        </div>
      </div>

      <div class="mobile-signature-footer">
        <button
          type="button"
          class="mobile-signature-btn mobile-signature-btn--secondary"
          data-action="start-over"
          aria-label="Start over - clear signature"
        >
          Start Over
        </button>
        <button
          type="button"
          class="mobile-signature-btn mobile-signature-btn--secondary"
          data-action="undo"
          aria-label="Undo last stroke"
        >
          Undo
        </button>
        <button
          type="button"
          class="mobile-signature-btn mobile-signature-btn--primary"
          data-action="done"
          aria-label="Done - save signature"
        >
          Done
        </button>
      </div>
    `,this.modalElement=e,this.canvasElement=e.querySelector(".mobile-signature-canvas"),e.querySelectorAll("[data-action]").forEach(n=>{let r=n.getAttribute("data-action");n.addEventListener("click",()=>this.handleAction(r))})}createRotateHint(){return`
      <div class="rotate-hint" role="status" aria-live="polite">
        <svg class="rotate-hint-icon" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <rect x="4" y="2" width="16" height="20" rx="2" ry="2"></rect>
          <line x1="12" y1="18" x2="12.01" y2="18"></line>
        </svg>
        <span>Rotate your device for a better signing experience</span>
      </div>
    `}injectStyles(){if(document.getElementById("mobile-signature-modal-styles"))return;let e=document.createElement("style");e.id="mobile-signature-modal-styles",e.textContent=`
      /* Mobile Signature Modal - Full Screen Overlay */
      .mobile-signature-modal {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        width: 100vw;
        height: 100vh;
        height: 100dvh; /* Dynamic viewport height for mobile browsers */
        background-color: var(--color-bg-primary, #ffffff);
        display: flex;
        flex-direction: column;
        z-index: 10000;
        opacity: 0;
        transform: translateY(100%);
        transition: opacity 0.2s ease, transform 0.2s ease;
        overflow: hidden;
        /* Safe area insets for notched devices */
        padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left);
      }

      .mobile-signature-modal--visible {
        opacity: 1;
        transform: translateY(0);
      }

      /* Header - Fixed at top */
      .mobile-signature-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 16px 20px;
        background-color: var(--color-bg-primary, #ffffff);
        border-bottom: 2px solid var(--color-text-secondary, #4a4a4a);
        flex-shrink: 0;
        min-height: 60px;
      }

      .mobile-signature-title {
        font-size: var(--font-size-xl, 28px);
        font-weight: 600;
        margin: 0;
        color: var(--color-text-primary, #1a1a1a);
      }

      .mobile-signature-close {
        width: 60px;
        height: 60px;
        min-width: 60px;
        min-height: 60px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: transparent;
        border: 2px solid var(--color-text-secondary, #4a4a4a);
        border-radius: var(--border-radius, 8px);
        cursor: pointer;
        color: var(--color-text-primary, #1a1a1a);
        padding: 0;
        transition: background-color 0.2s ease, border-color 0.2s ease;
      }

      .mobile-signature-close:hover,
      .mobile-signature-close:focus {
        background-color: var(--color-bg-secondary, #f8f8f8);
        border-color: var(--color-action-bg, #0056b3);
      }

      .mobile-signature-close:focus {
        outline: var(--focus-ring-width, 4px) solid var(--focus-ring-color, #0066cc);
        outline-offset: var(--focus-ring-offset, 2px);
      }

      .mobile-signature-close svg {
        width: 28px;
        height: 28px;
      }

      /* Canvas Area - Fills available space */
      .mobile-signature-canvas-area {
        flex: 1;
        display: flex;
        flex-direction: column;
        padding: 16px;
        min-height: 0;
        overflow: hidden;
      }

      .mobile-signature-instructions {
        text-align: center;
        font-size: var(--font-size-lg, 22px);
        color: var(--color-text-secondary, #4a4a4a);
        margin: 0 0 12px 0;
        flex-shrink: 0;
      }

      .mobile-signature-canvas-wrapper {
        flex: 1;
        position: relative;
        border: 3px solid var(--color-text-secondary, #4a4a4a);
        border-radius: var(--border-radius-lg, 12px);
        background-color: #ffffff;
        overflow: hidden;
        min-height: 150px;
      }

      .mobile-signature-canvas {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        cursor: crosshair;
        touch-action: none;
        background-color: transparent;
      }

      .mobile-signature-canvas:focus {
        outline: var(--focus-ring-width, 4px) solid var(--focus-ring-color, #0066cc);
        outline-offset: -4px;
      }

      .mobile-signature-baseline {
        position: absolute;
        bottom: 30%;
        left: 16px;
        right: 16px;
        border-top: 2px dashed var(--color-text-secondary, #4a4a4a);
        padding-top: 4px;
        text-align: center;
        font-size: var(--font-size-sm, 16px);
        color: var(--color-text-secondary, #4a4a4a);
        pointer-events: none;
        opacity: 0.6;
      }

      /* Rotate Hint */
      .rotate-hint {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 12px;
        padding: 12px 16px;
        margin-bottom: 12px;
        background-color: var(--color-warning-bg, #fef3cd);
        border: 2px solid var(--color-warning-border, #d4a200);
        border-radius: var(--border-radius, 8px);
        font-size: var(--font-size-base, 18px);
        color: var(--color-warning, #8a5700);
        text-align: center;
        flex-shrink: 0;
      }

      .rotate-hint-icon {
        flex-shrink: 0;
        animation: rotate-hint-wobble 1.5s ease-in-out infinite;
      }

      @keyframes rotate-hint-wobble {
        0%, 100% { transform: rotate(-10deg); }
        50% { transform: rotate(10deg); }
      }

      /* Hide rotate hint in landscape */
      @media (orientation: landscape) {
        .rotate-hint {
          display: none;
        }
      }

      /* Footer - Fixed at bottom */
      .mobile-signature-footer {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: var(--button-gap, 24px);
        padding: 16px 20px;
        background-color: var(--color-bg-primary, #ffffff);
        border-top: 2px solid var(--color-text-secondary, #4a4a4a);
        flex-shrink: 0;
        flex-wrap: wrap;
      }

      .mobile-signature-btn {
        min-width: 100px;
        min-height: 60px;
        height: 60px;
        padding: 12px 24px;
        font-size: var(--font-size-action, 24px);
        font-weight: 600;
        border-radius: var(--border-radius, 8px);
        cursor: pointer;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        transition: background-color 0.2s ease, transform 0.1s ease;
        white-space: nowrap;
      }

      .mobile-signature-btn:active {
        transform: scale(0.98);
      }

      .mobile-signature-btn:focus {
        outline: var(--focus-ring-width, 4px) solid var(--focus-ring-color, #0066cc);
        outline-offset: var(--focus-ring-offset, 2px);
      }

      .mobile-signature-btn--secondary {
        background-color: var(--color-bg-primary, #ffffff);
        color: var(--color-action-bg, #0056b3);
        border: 2px solid var(--color-action-bg, #0056b3);
      }

      .mobile-signature-btn--secondary:hover {
        background-color: var(--color-bg-secondary, #f8f8f8);
      }

      .mobile-signature-btn--primary {
        background-color: var(--color-action-bg, #0056b3);
        color: var(--color-action-text, #ffffff);
        border: 2px solid var(--color-action-border, #003d82);
      }

      .mobile-signature-btn--primary:hover {
        background-color: var(--color-action-bg-hover, #003d82);
      }

      /* High Contrast Mode Support */
      @media (prefers-contrast: high) {
        .mobile-signature-modal {
          background-color: #ffffff;
        }

        .mobile-signature-header,
        .mobile-signature-footer {
          border-color: #000000;
        }

        .mobile-signature-canvas-wrapper {
          border-color: #000000;
          border-width: 4px;
        }

        .mobile-signature-btn {
          border-width: 3px;
        }

        .mobile-signature-close {
          border-width: 3px;
        }
      }

      /* Dark Mode Support */
      @media (prefers-color-scheme: dark) {
        .mobile-signature-modal {
          background-color: var(--color-bg-primary, #1a1a1a);
        }

        .mobile-signature-canvas-wrapper {
          background-color: #ffffff;
        }
      }

      /* Responsive adjustments for landscape on mobile */
      @media (orientation: landscape) and (max-height: 500px) {
        .mobile-signature-header {
          padding: 8px 16px;
          min-height: 50px;
        }

        .mobile-signature-title {
          font-size: 20px;
        }

        .mobile-signature-close {
          width: 50px;
          height: 50px;
          min-width: 50px;
          min-height: 50px;
        }

        .mobile-signature-canvas-area {
          padding: 8px 16px;
        }

        .mobile-signature-instructions {
          font-size: 16px;
          margin-bottom: 8px;
        }

        .mobile-signature-footer {
          padding: 8px 16px;
          gap: 16px;
        }

        .mobile-signature-btn {
          min-height: 50px;
          height: 50px;
          padding: 8px 20px;
          font-size: 18px;
        }
      }

      /* Very small screens */
      @media (max-width: 360px) {
        .mobile-signature-footer {
          gap: 12px;
        }

        .mobile-signature-btn {
          padding: 8px 16px;
          min-width: 80px;
        }
      }
    `,document.head.appendChild(e)}initializeCanvas(){if(!this.canvasElement)return;let e=this.canvasElement.parentElement;if(!e)return;let t=e.getBoundingClientRect(),n=window.devicePixelRatio||1;this.canvasElement.width=t.width*n,this.canvasElement.height=t.height*n,this.ctx=this.canvasElement.getContext("2d"),this.ctx&&(this.ctx.scale(n,n),this.ctx.strokeStyle=this.penColor,this.ctx.lineWidth=this.penWidth,this.ctx.lineCap="round",this.ctx.lineJoin="round",this.ctx.fillStyle=this.penColor,this.clearCanvas(),this.redrawStrokes())}addEventListeners(){document.addEventListener("keydown",this.handleKeyDown),window.addEventListener("resize",this.handleResize),window.addEventListener("orientationchange",this.handleOrientationChange),this.canvasElement&&(this.canvasElement.addEventListener("touchstart",this.handleTouchStart,{passive:!1}),this.canvasElement.addEventListener("touchmove",this.handleTouchMove,{passive:!1}),this.canvasElement.addEventListener("touchend",this.handleTouchEnd,{passive:!1}),this.canvasElement.addEventListener("touchcancel",this.handleTouchEnd,{passive:!1}),this.canvasElement.addEventListener("mousedown",this.handleMouseDown),this.canvasElement.addEventListener("mousemove",this.handleMouseMove),this.canvasElement.addEventListener("mouseup",this.handleMouseUp),this.canvasElement.addEventListener("mouseleave",this.handleMouseUp))}removeEventListeners(){document.removeEventListener("keydown",this.handleKeyDown),window.removeEventListener("resize",this.handleResize),window.removeEventListener("orientationchange",this.handleOrientationChange),this.canvasElement&&(this.canvasElement.removeEventListener("touchstart",this.handleTouchStart),this.canvasElement.removeEventListener("touchmove",this.handleTouchMove),this.canvasElement.removeEventListener("touchend",this.handleTouchEnd),this.canvasElement.removeEventListener("touchcancel",this.handleTouchEnd),this.canvasElement.removeEventListener("mousedown",this.handleMouseDown),this.canvasElement.removeEventListener("mousemove",this.handleMouseMove),this.canvasElement.removeEventListener("mouseup",this.handleMouseUp),this.canvasElement.removeEventListener("mouseleave",this.handleMouseUp)),this.animationFrameId!==null&&(cancelAnimationFrame(this.animationFrameId),this.animationFrameId=null)}setupFocusTrap(){if(this.modalElement&&(this.focusableElements=Array.from(this.modalElement.querySelectorAll('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])')).filter(e=>!e.hasAttribute("disabled")&&e.offsetParent!==null),this.focusableElements.length>0)){this.firstFocusable=this.focusableElements[0],this.lastFocusable=this.focusableElements[this.focusableElements.length-1];let e=this.modalElement.querySelector(".mobile-signature-canvas");e?e.focus():this.firstFocusable.focus()}}handleKeyDown(e){if(e.key==="Escape"){e.preventDefault(),this.close();return}e.key==="Tab"&&this.firstFocusable&&this.lastFocusable&&(e.shiftKey?document.activeElement===this.firstFocusable&&(e.preventDefault(),this.lastFocusable.focus()):document.activeElement===this.lastFocusable&&(e.preventDefault(),this.firstFocusable.focus()))}handleResize(){this.resizeTimeout!==null&&clearTimeout(this.resizeTimeout),this.resizeTimeout=window.setTimeout(()=>{this.initializeCanvas(),this.updateRotateHint(),this.resizeTimeout=null},150)}handleOrientationChange(){setTimeout(()=>{this.initializeCanvas(),this.updateRotateHint()},100)}updateRotateHint(){if(!this.modalElement)return;let e=this.modalElement.querySelector(".rotate-hint"),t=e!==null,n=this.isPortrait();if(n&&!t){let r=this.modalElement.querySelector(".mobile-signature-canvas-area"),o=this.modalElement.querySelector(".mobile-signature-canvas-wrapper");if(r&&o){let s=document.createElement("div");s.innerHTML=this.createRotateHint(),r.insertBefore(s.firstElementChild,o)}}else!n&&t&&e?.remove()}isPortrait(){return window.innerHeight>window.innerWidth}handleTouchStart(e){if(e.preventDefault(),this.activeTouchId!==null)return;let t=e.touches[0];if(t.radiusX&&t.radiusY&&(t.radiusX+t.radiusY)/2>30)return;this.activeTouchId=t.identifier;let n=this.getTouchPos(t);this.startStroke(n)}handleTouchMove(e){if(e.preventDefault(),this.activeTouchId===null)return;let t=null;for(let r=0;r<e.touches.length;r++)if(e.touches[r].identifier===this.activeTouchId){t=e.touches[r];break}if(!t)return;let n=this.getTouchPos(t);this.continueStroke(n)}handleTouchEnd(e){e.preventDefault();let t=!0;for(let n=0;n<e.touches.length;n++)if(e.touches[n].identifier===this.activeTouchId){t=!1;break}t&&(this.endStroke(),this.activeTouchId=null)}handleMouseDown(e){let t=this.getMousePos(e);this.startStroke(t)}handleMouseMove(e){if(!this.isDrawing)return;let t=this.getMousePos(e);this.continueStroke(t)}handleMouseUp(){this.endStroke()}getTouchPos(e){if(!this.canvasElement)return{x:0,y:0};let t=this.canvasElement.getBoundingClientRect();return{x:e.clientX-t.left,y:e.clientY-t.top,pressure:e.force||.5}}getMousePos(e){if(!this.canvasElement)return{x:0,y:0};let t=this.canvasElement.getBoundingClientRect();return{x:e.clientX-t.left,y:e.clientY-t.top,pressure:.5}}startStroke(e){this.isDrawing=!0,this.points=[e],this.currentStroke={points:[e],color:this.penColor,width:this.penWidth},this.ctx&&(this.ctx.beginPath(),this.ctx.arc(e.x,e.y,this.penWidth/2,0,Math.PI*2),this.ctx.fill())}continueStroke(e){!this.isDrawing||!this.ctx||!this.currentStroke||(this.points.push(e),this.currentStroke.points.push(e),this.animationFrameId===null&&(this.animationFrameId=requestAnimationFrame(()=>{this.renderPendingPoints(),this.animationFrameId=null})))}renderPendingPoints(){if(!(!this.ctx||this.points.length<2))if(this.points.length>=3){let e=this.points[this.points.length-3],t=this.points[this.points.length-2],n=this.points[this.points.length-1],r=(t.x+n.x)/2,o=(t.y+n.y)/2;this.ctx.beginPath(),this.ctx.moveTo(e.x,e.y),this.ctx.quadraticCurveTo(t.x,t.y,r,o),this.ctx.stroke()}else{let e=this.points[this.points.length-2],t=this.points[this.points.length-1];this.ctx.beginPath(),this.ctx.moveTo(e.x,e.y),this.ctx.lineTo(t.x,t.y),this.ctx.stroke()}}endStroke(){this.isDrawing&&(this.isDrawing=!1,this.currentStroke&&this.currentStroke.points.length>0&&this.strokes.push(this.currentStroke),this.currentStroke=null,this.points=[])}clearCanvas(){if(!this.ctx||!this.canvasElement)return;let e=window.devicePixelRatio||1;this.ctx.clearRect(0,0,this.canvasElement.width/e,this.canvasElement.height/e)}redrawStrokes(){if(this.ctx){for(let e of this.strokes){if(this.ctx.strokeStyle=e.color,this.ctx.lineWidth=e.width,this.ctx.fillStyle=e.color,e.points.length===0)continue;let t=e.points[0];if(this.ctx.beginPath(),this.ctx.arc(t.x,t.y,e.width/2,0,Math.PI*2),this.ctx.fill(),e.points.length>=2){for(let n=2;n<e.points.length;n++){let r=e.points[n-2],o=e.points[n-1],s=e.points[n],a=(o.x+s.x)/2,l=(o.y+s.y)/2;this.ctx.beginPath(),this.ctx.moveTo(r.x,r.y),this.ctx.quadraticCurveTo(o.x,o.y,a,l),this.ctx.stroke()}e.points.length===2&&(this.ctx.beginPath(),this.ctx.moveTo(e.points[0].x,e.points[0].y),this.ctx.lineTo(e.points[1].x,e.points[1].y),this.ctx.stroke())}}this.ctx.strokeStyle=this.penColor,this.ctx.lineWidth=this.penWidth,this.ctx.fillStyle=this.penColor}}handleAction(e){switch(e){case"close":this.close();break;case"start-over":this.strokes=[],this.clearCanvas();break;case"undo":this.strokes.pop(),this.clearCanvas(),this.redrawStrokes();break;case"done":this.saveAndClose();break}}saveAndClose(){if(!this.canvasElement){this.close();return}if(this.strokes.length===0){this.showValidationError("Please draw your signature first");return}let t={dataUrl:this.canvasElement.toDataURL("image/png"),type:"drawn",timestamp:new Date().toISOString()};this.hideModal(t)}showValidationError(e){alert(e)}escapeHtml(e){let t=document.createElement("div");return t.textContent=e,t.innerHTML}};function ze(){return window.matchMedia("(max-width: 768px)").matches}function qe(i){return new F(i)}typeof window<"u"&&(window.MobileSignatureModal=F,window.isMobileDevice=ze,window.createSignatureModal=qe);var N=class{constructor(e){this.guideCanvas=null;this.guideCtx=null;this.isDrawing=!1;this.currentStroke=[];this.strokes=[];this.redoStack=[];this.isTouchDevice=!1;this.onchange=null;this.container=e.container,this.height=e.height??200,this.strokeColor=e.strokeColor??"#000080",this.backgroundColor=e.backgroundColor??"#ffffff",this.showGuides=e.showGuides??!0,this.isTouchDevice="ontouchstart"in window||navigator.maxTouchPoints>0,this.strokeWidth=e.strokeWidth??(this.isTouchDevice?4:3);let t=this.container.clientWidth||400;this.width=e.width??t,this.wrapper=this.createWrapper(),this.showGuides&&(this.guideCanvas=this.createGuideCanvas(),this.guideCtx=this.guideCanvas.getContext("2d")),this.canvas=this.createCanvas();let n=this.canvas.getContext("2d");if(!n)throw new Error("Failed to get 2D context from canvas");this.ctx=n,this.guideCanvas&&this.wrapper.appendChild(this.guideCanvas),this.wrapper.appendChild(this.canvas),this.container.appendChild(this.wrapper),this.initializeCanvas(),this.showGuides&&this.drawGuides(),this.boundHandlers={mouseDown:this.handleMouseDown.bind(this),mouseMove:this.handleMouseMove.bind(this),mouseUp:this.handleMouseUp.bind(this),mouseLeave:this.handleMouseLeave.bind(this),touchStart:this.handleTouchStart.bind(this),touchMove:this.handleTouchMove.bind(this),touchEnd:this.handleTouchEnd.bind(this),resize:this.handleResize.bind(this)},this.attachEventListeners()}createWrapper(){let e=document.createElement("div");return e.className="signature-capture-wrapper",e.style.cssText=`
      position: relative;
      width: 100%;
      max-width: ${this.width}px;
      height: ${this.height}px;
      border: 3px solid var(--color-text-secondary, #4a4a4a);
      border-radius: var(--border-radius-lg, 12px);
      background-color: ${this.backgroundColor};
      overflow: hidden;
      touch-action: none;
      user-select: none;
      -webkit-user-select: none;
    `,e}createGuideCanvas(){let e=document.createElement("canvas");return e.className="signature-capture-guides",e.width=this.width,e.height=this.height,e.style.cssText=`
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      pointer-events: none;
    `,e}createCanvas(){let e=document.createElement("canvas");return e.className="signature-capture-canvas",e.width=this.width,e.height=this.height,e.style.cssText=`
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      cursor: crosshair;
      touch-action: none;
    `,e.setAttribute("role","img"),e.setAttribute("aria-label","Signature drawing area. Use mouse or touch to draw your signature."),e.tabIndex=0,e}initializeCanvas(){this.ctx.fillStyle=this.backgroundColor,this.ctx.fillRect(0,0,this.canvas.width,this.canvas.height),this.ctx.strokeStyle=this.strokeColor,this.ctx.lineWidth=this.strokeWidth,this.ctx.lineCap="round",this.ctx.lineJoin="round"}drawGuides(){if(!this.guideCtx||!this.guideCanvas)return;let e=this.guideCtx,t=this.guideCanvas.width,n=this.guideCanvas.height;e.clearRect(0,0,t,n);let r=n*.7;e.strokeStyle="#e0e0e0",e.lineWidth=2,e.setLineDash([10,5]),e.beginPath(),e.moveTo(20,r),e.lineTo(t-20,r),e.stroke(),e.setLineDash([]),e.fillStyle="#cccccc",e.font="italic 16px var(--font-family-body, sans-serif)",e.textAlign="center",e.fillText("Sign on the line above",t/2,n-15)}attachEventListeners(){this.canvas.addEventListener("mousedown",this.boundHandlers.mouseDown),this.canvas.addEventListener("mousemove",this.boundHandlers.mouseMove),this.canvas.addEventListener("mouseup",this.boundHandlers.mouseUp),this.canvas.addEventListener("mouseleave",this.boundHandlers.mouseLeave),this.canvas.addEventListener("touchstart",this.boundHandlers.touchStart,{passive:!1}),this.canvas.addEventListener("touchmove",this.boundHandlers.touchMove,{passive:!1}),this.canvas.addEventListener("touchend",this.boundHandlers.touchEnd),window.addEventListener("resize",this.boundHandlers.resize)}handleResize(){let e=this.container.clientWidth;if(e>0&&e!==this.width){let t=[...this.strokes];this.width=e,this.canvas.width=this.width,this.guideCanvas&&(this.guideCanvas.width=this.width),this.initializeCanvas(),this.showGuides&&this.drawGuides(),this.strokes=t,this.redrawAllStrokes()}}getMousePoint(e){let t=this.canvas.getBoundingClientRect(),n=this.canvas.width/t.width,r=this.canvas.height/t.height;return{x:(e.clientX-t.left)*n,y:(e.clientY-t.top)*r,pressure:.5}}getTouchPoint(e){let t=this.canvas.getBoundingClientRect(),n=this.canvas.width/t.width,r=this.canvas.height/t.height,o=e.touches[0],s=.5;return"force"in o&&typeof o.force=="number"&&(s=o.force),{x:(o.clientX-t.left)*n,y:(o.clientY-t.top)*r,pressure:s}}startDrawing(e){this.isDrawing=!0,this.currentStroke=[e],this.redoStack=[],this.ctx.beginPath(),this.ctx.moveTo(e.x,e.y)}continueDrawing(e){if(!this.isDrawing)return;this.currentStroke.push(e);let t=e.pressure??.5,n=this.strokeWidth*(.5+t);this.ctx.lineWidth=n,this.ctx.lineTo(e.x,e.y),this.ctx.stroke(),this.ctx.beginPath(),this.ctx.moveTo(e.x,e.y)}endDrawing(){this.isDrawing&&(this.isDrawing=!1,this.currentStroke.length>0&&(this.strokes.push({points:this.currentStroke,color:this.strokeColor,width:this.strokeWidth}),this.currentStroke=[],this.notifyChange()))}handleMouseDown(e){e.preventDefault();let t=this.getMousePoint(e);this.startDrawing(t)}handleMouseMove(e){e.preventDefault();let t=this.getMousePoint(e);this.continueDrawing(t)}handleMouseUp(){this.endDrawing()}handleMouseLeave(){this.endDrawing()}handleTouchStart(e){if(e.preventDefault(),e.touches.length===1){let t=this.getTouchPoint(e);this.startDrawing(t)}}handleTouchMove(e){if(e.preventDefault(),e.touches.length===1){let t=this.getTouchPoint(e);this.continueDrawing(t)}}handleTouchEnd(){this.endDrawing()}redrawAllStrokes(){this.ctx.fillStyle=this.backgroundColor,this.ctx.fillRect(0,0,this.canvas.width,this.canvas.height);for(let e of this.strokes){if(e.points.length===0)continue;this.ctx.strokeStyle=e.color,this.ctx.lineWidth=e.width,this.ctx.beginPath();let t=e.points[0];this.ctx.moveTo(t.x,t.y);for(let n=1;n<e.points.length;n++){let r=e.points[n],o=r.pressure??.5;this.ctx.lineWidth=e.width*(.5+o),this.ctx.lineTo(r.x,r.y),this.ctx.stroke(),this.ctx.beginPath(),this.ctx.moveTo(r.x,r.y)}}this.ctx.strokeStyle=this.strokeColor,this.ctx.lineWidth=this.strokeWidth}notifyChange(){this.onchange&&this.onchange(this.isEmpty())}clear(){this.redoStack=[...this.strokes],this.strokes=[],this.currentStroke=[],this.redrawAllStrokes(),this.notifyChange(),this.announceToScreenReader("Signature cleared")}undo(){if(this.strokes.length===0)return;let e=this.strokes.pop();e&&this.redoStack.push(e),this.redrawAllStrokes(),this.notifyChange(),this.announceToScreenReader("Stroke undone")}redo(){if(this.redoStack.length===0)return;let e=this.redoStack.pop();e&&this.strokes.push(e),this.redrawAllStrokes(),this.notifyChange(),this.announceToScreenReader("Stroke restored")}isEmpty(){return this.strokes.length===0}canUndo(){return this.strokes.length>0}canRedo(){return this.redoStack.length>0}getStrokeCount(){return this.strokes.length}toDataURL(e="png"){return e==="svg"?this.toSVG():this.canvas.toDataURL("image/png")}toSVG(){let e=this.canvas.width,t=this.canvas.height,n="";for(let o of this.strokes){if(o.points.length===0)continue;let s=o.points[0];n+=`M ${s.x} ${s.y} `;for(let a=1;a<o.points.length;a++){let l=o.points[a];n+=`L ${l.x} ${l.y} `}}let r=`<svg xmlns="http://www.w3.org/2000/svg" width="${e}" height="${t}" viewBox="0 0 ${e} ${t}">
      <rect width="${e}" height="${t}" fill="${this.backgroundColor}"/>
      <path d="${n}" stroke="${this.strokeColor}" stroke-width="${this.strokeWidth}" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`;return`data:image/svg+xml;base64,${btoa(r)}`}toBlob(){return new Promise((e,t)=>{this.canvas.toBlob(n=>{n?e(n):t(new Error("Failed to create blob from canvas"))},"image/png",1)})}getStrokes(){return[...this.strokes]}loadStrokes(e){this.strokes=e.map(t=>({points:[...t.points],color:t.color,width:t.width})),this.redoStack=[],this.redrawAllStrokes(),this.notifyChange()}announceToScreenReader(e){let t=document.createElement("div");t.setAttribute("role","status"),t.setAttribute("aria-live","polite"),t.className="visually-hidden",t.textContent=e,document.body.appendChild(t),setTimeout(()=>{t.remove()},1e3)}destroy(){this.canvas.removeEventListener("mousedown",this.boundHandlers.mouseDown),this.canvas.removeEventListener("mousemove",this.boundHandlers.mouseMove),this.canvas.removeEventListener("mouseup",this.boundHandlers.mouseUp),this.canvas.removeEventListener("mouseleave",this.boundHandlers.mouseLeave),this.canvas.removeEventListener("touchstart",this.boundHandlers.touchStart),this.canvas.removeEventListener("touchmove",this.boundHandlers.touchMove),this.canvas.removeEventListener("touchend",this.boundHandlers.touchEnd),window.removeEventListener("resize",this.boundHandlers.resize),this.wrapper.remove(),this.strokes=[],this.redoStack=[],this.onchange=null}getCanvas(){return this.canvas}getWrapper(){return this.wrapper}};var j=class{constructor(e,t={}){this.mode="draw";this.currentFieldId=null;this.drawTab=null;this.typeTab=null;this.drawPanel=null;this.typePanel=null;this.drawTabBtn=null;this.typeTabBtn=null;this.canvas=null;this.ctx=null;this.clearBtn=null;this.applyBtn=null;this.cancelBtn=null;this.closeBtn=null;this.typedSignature=null;this.typedSignatureContainer=null;this.isDrawing=!1;this.lastX=0;this.lastY=0;this.hasDrawn=!1;this.modal=e,this.onApply=t.onApply||(()=>{}),this.onCancel=t.onCancel||(()=>{}),this.penColor=t.penColor||"#000000",this.penWidth=t.penWidth||2,this.initializeElements(),this.bindEvents(),this.enhanceTypeTab()}initializeElements(){this.drawTabBtn=this.modal.querySelector('[data-tab="draw"]'),this.typeTabBtn=this.modal.querySelector('[data-tab="type"]'),this.drawPanel=this.modal.querySelector("#draw-tab"),this.typePanel=this.modal.querySelector("#type-tab"),this.canvas=this.modal.querySelector("#signature-pad"),this.clearBtn=this.modal.querySelector("#clear-signature"),this.applyBtn=this.modal.querySelector("#apply-signature"),this.cancelBtn=this.modal.querySelector("#cancel-signature"),this.closeBtn=this.modal.querySelector("#close-signature-modal"),this.canvas&&(this.ctx=this.canvas.getContext("2d"))}bindEvents(){this.drawTabBtn?.addEventListener("click",()=>this.switchTab("draw")),this.typeTabBtn?.addEventListener("click",()=>this.switchTab("type")),this.clearBtn?.addEventListener("click",()=>this.clearCanvas()),this.applyBtn?.addEventListener("click",()=>this.apply()),this.cancelBtn?.addEventListener("click",()=>this.hide()),this.closeBtn?.addEventListener("click",()=>this.hide()),this.modal.addEventListener("click",e=>{e.target===this.modal&&this.hide()}),document.addEventListener("keydown",e=>{e.key==="Escape"&&!this.modal.classList.contains("hidden")&&this.hide()}),this.canvas&&this.bindCanvasEvents()}bindCanvasEvents(){this.canvas&&(this.canvas.addEventListener("mousedown",this.handlePointerDown.bind(this)),this.canvas.addEventListener("mousemove",this.handlePointerMove.bind(this)),this.canvas.addEventListener("mouseup",this.handlePointerUp.bind(this)),this.canvas.addEventListener("mouseleave",this.handlePointerUp.bind(this)),this.canvas.addEventListener("touchstart",this.handleTouchStart.bind(this)),this.canvas.addEventListener("touchmove",this.handleTouchMove.bind(this)),this.canvas.addEventListener("touchend",this.handlePointerUp.bind(this)),this.canvas.addEventListener("touchstart",e=>e.preventDefault()),this.canvas.addEventListener("touchmove",e=>e.preventDefault()))}enhanceTypeTab(){this.typePanel&&(this.typePanel.innerHTML="",this.typedSignatureContainer=document.createElement("div"),this.typedSignatureContainer.id="typed-signature-container",this.typePanel.appendChild(this.typedSignatureContainer),this.typedSignature=new D({container:this.typedSignatureContainer,fonts:O.map(e=>e.name),defaultFont:"Dancing Script",fontSize:48,textColor:"#000080",backgroundColor:"#ffffff",placeholder:"Type your full name"}))}switchTab(e){this.mode=e,e==="draw"?(this.drawTabBtn?.classList.add("active"),this.typeTabBtn?.classList.remove("active"),this.drawPanel?.classList.add("active"),this.drawPanel?.classList.remove("hidden"),this.typePanel?.classList.remove("active"),this.typePanel?.classList.add("hidden"),this.initCanvas()):(this.typeTabBtn?.classList.add("active"),this.drawTabBtn?.classList.remove("active"),this.typePanel?.classList.add("active"),this.typePanel?.classList.remove("hidden"),this.drawPanel?.classList.remove("active"),this.drawPanel?.classList.add("hidden"))}initCanvas(){if(!this.canvas||!this.ctx)return;let e=this.canvas.getBoundingClientRect(),t=window.devicePixelRatio||1;this.canvas.width=e.width*t,this.canvas.height=e.height*t,this.ctx.scale(t,t);let n=window.innerWidth<768;this.ctx.strokeStyle=this.penColor,this.ctx.lineWidth=n?Math.max(this.penWidth,3):this.penWidth,this.ctx.lineCap="round",this.ctx.lineJoin="round",this.clearCanvas()}clearCanvas(){!this.canvas||!this.ctx||(this.ctx.fillStyle="#ffffff",this.ctx.fillRect(0,0,this.canvas.width,this.canvas.height),this.ctx.fillStyle=this.penColor,this.hasDrawn=!1)}getPointerPos(e){if(!this.canvas)return{x:0,y:0};let t=this.canvas.getBoundingClientRect();return{x:e.clientX-t.left,y:e.clientY-t.top}}getTouchPos(e){if(!this.canvas)return{x:0,y:0};let t=this.canvas.getBoundingClientRect(),n=e.touches[0];return{x:n.clientX-t.left,y:n.clientY-t.top}}handlePointerDown(e){this.isDrawing=!0;let t=this.getPointerPos(e);this.lastX=t.x,this.lastY=t.y,this.ctx&&(this.ctx.beginPath(),this.ctx.arc(t.x,t.y,this.penWidth/2,0,Math.PI*2),this.ctx.fill()),this.hasDrawn=!0}handlePointerMove(e){if(!this.isDrawing||!this.ctx)return;let t=this.getPointerPos(e);this.ctx.beginPath(),this.ctx.moveTo(this.lastX,this.lastY),this.ctx.lineTo(t.x,t.y),this.ctx.stroke(),this.lastX=t.x,this.lastY=t.y,this.hasDrawn=!0}handlePointerUp(){this.isDrawing=!1}handleTouchStart(e){this.isDrawing=!0;let t=this.getTouchPos(e);this.lastX=t.x,this.lastY=t.y,this.ctx&&(this.ctx.beginPath(),this.ctx.arc(t.x,t.y,this.penWidth/2,0,Math.PI*2),this.ctx.fill()),this.hasDrawn=!0}handleTouchMove(e){if(!this.isDrawing||!this.ctx)return;let t=this.getTouchPos(e);this.ctx.beginPath(),this.ctx.moveTo(this.lastX,this.lastY),this.ctx.lineTo(t.x,t.y),this.ctx.stroke(),this.lastX=t.x,this.lastY=t.y,this.hasDrawn=!0}isCanvasEmpty(){return!this.hasDrawn}apply(){let e,t,n;if(this.mode==="draw"){if(this.isCanvasEmpty()){alert("Please draw your signature");return}if(this.canvas)e=this.canvas.toDataURL("image/png");else return}else{if(!this.typedSignature||this.typedSignature.isEmpty()){alert("Please type your name");return}e=this.typedSignature.toDataURL(),t=this.typedSignature.getText(),n=this.typedSignature.getFont()}let r={fieldId:this.currentFieldId,signatureData:e,mode:this.mode,text:t,font:n};this.onApply(r),this.hide()}show(e){this.currentFieldId=e||null,this.modal.classList.remove("hidden");let t=this.modal.querySelector(".modal");t&&window.innerWidth<768&&t.classList.add("bottom-sheet-mobile"),this.switchTab("draw"),this.clearCanvas(),this.typedSignature&&this.typedSignature.setText("")}hide(){this.modal.classList.add("hidden"),this.onCancel()}destroy(){this.typedSignature&&(this.typedSignature.destroy(),this.typedSignature=null)}};function $e(i,e={}){return new j(i,e)}var B=class{constructor(e={}){this.overlay=null;this.modalEl=null;this.capture=null;this.btnStartOver=null;this.btnUndo=null;this.btnRedo=null;this.btnUseSignature=null;this.btnCancel=null;this.isOpenState=!1;this.previousActiveElement=null;this.boundKeydownHandler=null;this.title=e.title??"Draw Your Signature",this.instructions=e.instructions??"Use your finger or mouse to sign below. Take your time.",this.labels={startOver:e.labels?.startOver??"Start Over",undoStroke:e.labels?.undoStroke??"Undo Last Stroke",redoStroke:e.labels?.redoStroke??"Redo",useSignature:e.labels?.useSignature??"Use This Signature",cancel:e.labels?.cancel??"Cancel"},this.canvasHeight=e.canvasHeight??220,this.strokeColor=e.strokeColor??"#000080",this.showGuides=e.showGuides??!0,this.closeOnBackdrop=e.closeOnBackdrop??!0,this.closeOnEscape=e.closeOnEscape??!0,this.onAcceptCallback=e.onAccept??null,this.onCancelCallback=e.onCancel??null}createModalDOM(){this.overlay=document.createElement("div"),this.overlay.className="signature-capture-modal-overlay modal-overlay",this.overlay.setAttribute("role","dialog"),this.overlay.setAttribute("aria-modal","true"),this.overlay.setAttribute("aria-labelledby","sig-capture-modal-title"),this.overlay.setAttribute("aria-describedby","sig-capture-modal-instructions"),this.modalEl=document.createElement("div"),this.modalEl.className="signature-capture-modal modal-content",this.modalEl.style.cssText=`
      max-width: 600px;
      width: 90%;
      max-height: 90vh;
      overflow-y: auto;
      padding: var(--spacing-lg, 32px);
      display: flex;
      flex-direction: column;
      gap: var(--spacing-md, 24px);
      background-color: var(--color-bg-primary, #ffffff);
      border-radius: var(--border-radius-lg, 12px);
    `;let e=this.createHeader(),t=document.createElement("p");t.id="sig-capture-modal-instructions",t.className="signature-capture-modal-instructions",t.textContent=this.instructions,t.style.cssText=`
      font-size: var(--font-size-lg, 22px);
      color: var(--color-text-secondary, #4a4a4a);
      margin: 0;
      text-align: center;
      line-height: 1.5;
    `;let n=document.createElement("div");n.className="signature-capture-modal-pad",n.style.cssText=`
      width: 100%;
      min-height: ${this.canvasHeight}px;
    `;let r=this.createActionRow(),o=this.createBottomRow();this.modalEl.appendChild(e),this.modalEl.appendChild(t),this.modalEl.appendChild(n),this.modalEl.appendChild(r),this.modalEl.appendChild(o),this.overlay.appendChild(this.modalEl),this.btnUndo?.addEventListener("click",()=>this.handleUndo()),this.btnRedo?.addEventListener("click",()=>this.handleRedo()),this.btnStartOver?.addEventListener("click",()=>this.handleStartOver()),this.btnCancel?.addEventListener("click",()=>this.handleCancel()),this.btnUseSignature?.addEventListener("click",()=>this.handleAccept()),this.closeOnBackdrop&&this.overlay.addEventListener("click",s=>{s.target===this.overlay&&this.handleCancel()}),this.capture=new N({container:n,height:this.canvasHeight,strokeColor:this.strokeColor,showGuides:this.showGuides}),this.capture.onchange=()=>{this.updateButtonStates()}}createHeader(){let e=document.createElement("div");e.className="signature-capture-modal-header",e.style.cssText=`
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
    `;let t=document.createElement("h2");t.id="sig-capture-modal-title",t.className="signature-capture-modal-title",t.textContent=this.title,t.style.cssText=`
      font-size: var(--font-size-xl, 28px);
      font-weight: 700;
      margin: 0;
      color: var(--color-text-primary, #1a1a1a);
    `;let n=document.createElement("button");return n.className="signature-capture-modal-close",n.setAttribute("aria-label","Close signature dialog"),n.innerHTML=`
      <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    `,n.style.cssText=`
      background: none;
      border: none;
      cursor: pointer;
      padding: 8px;
      color: var(--color-text-secondary, #4a4a4a);
      min-width: 44px;
      min-height: 44px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: var(--border-radius, 8px);
      transition: background-color 0.2s;
    `,n.addEventListener("click",()=>this.handleCancel()),e.appendChild(t),e.appendChild(n),e}createActionRow(){let e=document.createElement("div");return e.className="signature-capture-modal-actions",e.style.cssText=`
      display: flex;
      justify-content: center;
      gap: var(--spacing-sm, 16px);
      flex-wrap: wrap;
    `,this.btnUndo=this.createButton(this.labels.undoStroke,"secondary","undo-stroke",`<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 7v6h6"></path>
        <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"></path>
      </svg>`),this.btnUndo.disabled=!0,this.btnRedo=this.createButton(this.labels.redoStroke,"secondary","redo-stroke",`<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21 7v6h-6"></path>
        <path d="M3 17a9 9 0 0 1 9-9 9 9 0 0 1 6 2.3L21 13"></path>
      </svg>`),this.btnRedo.disabled=!0,this.btnStartOver=this.createButton(this.labels.startOver,"secondary","start-over",`<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M2.5 2v6h6M2.66 15.57a10 10 0 1 0 .57-8.38"></path>
      </svg>`),this.btnStartOver.disabled=!0,e.appendChild(this.btnUndo),e.appendChild(this.btnRedo),e.appendChild(this.btnStartOver),e}createBottomRow(){let e=document.createElement("div");return e.className="signature-capture-modal-bottom",e.style.cssText=`
      display: flex;
      justify-content: center;
      gap: var(--button-gap, 24px);
      flex-wrap: wrap;
      margin-top: var(--spacing-sm, 16px);
    `,this.btnCancel=this.createButton(this.labels.cancel,"secondary","cancel"),this.btnCancel.style.minWidth="140px",this.btnUseSignature=this.createButton(this.labels.useSignature,"primary","use-signature",`<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="20 6 9 17 4 12"></polyline>
      </svg>`),this.btnUseSignature.classList.add("btn-large"),this.btnUseSignature.style.cssText+=`
      min-width: 220px;
      font-size: var(--font-size-xl, 28px);
    `,this.btnUseSignature.disabled=!0,e.appendChild(this.btnCancel),e.appendChild(this.btnUseSignature),e}createButton(e,t,n,r){let o=document.createElement("button");return o.className=`btn-${t} signature-capture-modal-btn`,o.setAttribute("data-action",n),o.style.cssText=`
      min-height: 60px;
      padding: var(--spacing-sm, 16px) var(--spacing-md, 24px);
      font-size: var(--font-size-action, 24px);
      font-weight: 600;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      border-radius: var(--border-radius, 8px);
      cursor: pointer;
      transition: all 0.2s;
    `,t==="primary"?o.style.cssText+=`
        background-color: var(--color-action-bg, #0056b3);
        color: var(--color-action-text, #ffffff);
        border: 2px solid var(--color-action-border, #003d82);
      `:o.style.cssText+=`
        background-color: var(--color-bg-primary, #ffffff);
        color: var(--color-action-bg, #0056b3);
        border: 2px solid var(--color-action-bg, #0056b3);
      `,r?o.innerHTML=`<span class="btn-icon" aria-hidden="true">${r}</span><span>${e}</span>`:o.textContent=e,o}updateButtonStates(){if(!this.capture)return;let e=this.capture.isEmpty(),t=this.capture.canUndo(),n=this.capture.canRedo();this.btnUndo&&(this.btnUndo.disabled=!t),this.btnRedo&&(this.btnRedo.disabled=!n),this.btnStartOver&&(this.btnStartOver.disabled=e),this.btnUseSignature&&(this.btnUseSignature.disabled=e)}handleKeydown(e){if(e.key==="Escape"&&this.closeOnEscape&&(e.preventDefault(),this.handleCancel()),e.key==="Tab"&&this.modalEl){let t=this.modalEl.querySelectorAll('button:not([disabled]), [tabindex]:not([tabindex="-1"])'),n=t[0],r=t[t.length-1];e.shiftKey&&document.activeElement===n?(e.preventDefault(),r?.focus()):!e.shiftKey&&document.activeElement===r&&(e.preventDefault(),n?.focus())}(e.ctrlKey||e.metaKey)&&e.key==="z"&&(e.preventDefault(),e.shiftKey?this.handleRedo():this.handleUndo())}handleUndo(){this.capture?.undo(),this.updateButtonStates()}handleRedo(){this.capture?.redo(),this.updateButtonStates()}handleStartOver(){this.capture?.clear(),this.updateButtonStates()}handleAccept(){if(!this.capture||this.capture.isEmpty())return;let e=this.capture.toDataURL("png"),t=this.capture.getStrokes();this.close(),this.onAcceptCallback&&this.onAcceptCallback(e,t)}handleCancel(){this.close(),this.onCancelCallback&&this.onCancelCallback()}announceToScreenReader(e){let t=document.createElement("div");t.setAttribute("role","status"),t.setAttribute("aria-live","polite"),t.className="visually-hidden",t.textContent=e,document.body.appendChild(t),setTimeout(()=>{t.remove()},1e3)}open(){this.isOpenState||(this.previousActiveElement=document.activeElement,this.overlay||this.createModalDOM(),this.overlay&&document.body.appendChild(this.overlay),document.body.style.overflow="hidden",this.boundKeydownHandler=this.handleKeydown.bind(this),document.addEventListener("keydown",this.boundKeydownHandler),setTimeout(()=>{this.capture&&this.capture.getCanvas().focus()},100),this.isOpenState=!0,this.announceToScreenReader("Signature dialog opened. Draw your signature."))}close(){this.isOpenState&&(this.boundKeydownHandler&&(document.removeEventListener("keydown",this.boundKeydownHandler),this.boundKeydownHandler=null),document.body.style.overflow="",this.overlay&&this.overlay.remove(),this.capture&&(this.capture.destroy(),this.capture=null),this.overlay=null,this.modalEl=null,this.btnUndo=null,this.btnRedo=null,this.btnStartOver=null,this.btnCancel=null,this.btnUseSignature=null,this.previousActiveElement instanceof HTMLElement&&this.previousActiveElement.focus(),this.isOpenState=!1,this.announceToScreenReader("Signature dialog closed"))}isOpen(){return this.isOpenState}getCapture(){return this.capture}destroy(){this.close(),this.onAcceptCallback=null,this.onCancelCallback=null}};function We(i){return new B(i)}typeof window<"u"&&(window.SignatureModal=j,window.initSignatureModal=$e,window.SignatureCaptureModal=B,window.createSignatureCaptureModal=We);var d=g("Auth"),le="docsign_access_token",Ke="docsign_refresh_token",Je="docsign_user",q="https://api.getsignatures.org",ae=[];function Pt(i){return ae.push(i),()=>{let e=ae.indexOf(i);e>-1&&ae.splice(e,1)}}function Lt(){let i={isAuthenticated:Q(),user:ce()};ae.forEach(e=>e(i))}function z(){try{return localStorage.getItem(le)}catch(i){return d.warn("Failed to get access token:",i),null}}function bn(){try{return localStorage.getItem(Ke)}catch(i){return d.warn("Failed to get refresh token:",i),null}}function wn(i,e){try{localStorage.setItem(le,i),localStorage.setItem(Ke,e)}catch(t){d.error("Failed to store tokens:",t)}}function Sn(){try{localStorage.removeItem(le),localStorage.removeItem(Ke),localStorage.removeItem(Je)}catch(i){d.error("Failed to clear tokens:",i)}}function xn(i){try{localStorage.setItem(Je,JSON.stringify(i))}catch(e){d.error("Failed to store user:",e)}}function ce(){try{let i=localStorage.getItem(Je);return i?JSON.parse(i):null}catch(i){return d.warn("Failed to get user:",i),null}}function Q(){return z()!==null}function Mt(){let i=ce();return i?i.daily_documents_remaining:0}async function Dt(i,e,t){try{d.info("Registering new user:",i);let n=await fetch(`${q}/auth/register`,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({email:i,password:e,name:t})}),r=await n.json();return n.ok?(d.info("Registration successful, verification email sent"),{success:!0,user_id:r.user_id,message:r.message}):(d.warn("Registration failed:",r.message),{success:!1,message:r.message||"Registration failed",error:r.message})}catch(n){return d.error("Registration error:",n),{success:!1,message:"Network error. Please check your connection and try again.",error:String(n)}}}async function Rt(i,e){try{d.info("Logging in user:",i);let t=await fetch(`${q}/auth/login`,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({email:i,password:e})}),n=await t.json();return t.ok?(n.access_token&&n.refresh_token&&n.user&&(wn(n.access_token,n.refresh_token),xn(n.user),Lt(),d.info("Login successful:",n.user.email)),{success:!0,access_token:n.access_token,refresh_token:n.refresh_token,expires_in:n.expires_in,user:n.user}):(d.warn("Login failed:",n.error),{success:!1,error:n.error||"Login failed"})}catch(t){return d.error("Login error:",t),{success:!1,error:"Network error. Please check your connection and try again."}}}async function Ye(){let i=z();if(i)try{await fetch(`${q}/auth/logout`,{method:"POST",headers:{Authorization:`Bearer ${i}`,"Content-Type":"application/json"}})}catch(e){d.warn("Logout API call failed:",e)}Sn(),Lt(),d.info("User logged out")}async function Ge(){let i=bn();if(!i)return d.debug("No refresh token available"),!1;try{d.debug("Refreshing access token");let e=await fetch(`${q}/auth/refresh`,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({refresh_token:i})});if(!e.ok)return d.warn("Token refresh failed, logging out"),await Ye(),!1;let t=await e.json();return t.access_token?(localStorage.setItem(le,t.access_token),d.debug("Token refreshed successfully"),!0):!1}catch(e){return d.error("Token refresh error:",e),!1}}async function At(i,e={}){let t=z();if(!t)throw new Error("Not authenticated");let n=new Headers(e.headers);n.set("Authorization",`Bearer ${t}`);let r=await fetch(i,{...e,headers:n});return r.status===401&&await Ge()&&(t=z(),t&&(n.set("Authorization",`Bearer ${t}`),r=await fetch(i,{...e,headers:n}))),r}async function It(i){try{d.info("Requesting password reset for:",i);let e=await fetch(`${q}/auth/forgot-password`,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({email:i})}),t=await e.json();return e.ok?{success:!0,message:t.message||"A password reset link has been sent to your email."}:(d.warn("Forgot password failed:",t.message),{success:!1,message:t.message||"Unable to process request. Please try again."})}catch(e){return d.error("Forgot password error:",e),{success:!1,message:"Network error. Please try again."}}}async function Ot(i,e){try{d.info("Resetting password");let t=await fetch(`${q}/auth/reset-password`,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({token:i,new_password:e})}),n=await t.json();return t.ok?(d.info("Password reset successful"),{success:!0,message:n.message||"Password reset successfully"}):{success:!1,message:n.message||"Password reset failed"}}catch(t){return d.error("Reset password error:",t),{success:!1,message:"Network error. Please try again."}}}function Ft(i){return i.length<8?"Password must be at least 8 characters long":/[A-Z]/.test(i)?/[a-z]/.test(i)?/[0-9]/.test(i)?null:"Password must contain at least one number":"Password must contain at least one lowercase letter":"Password must contain at least one uppercase letter"}function Nt(i){let e=i.trim();if(e.length<5)return"Email is too short";if(!e.includes("@"))return"Please enter a valid email address";let t=e.split("@");return t.length!==2||!t[0]||!t[1].includes(".")?"Please enter a valid email address":null}function Bt(){if(typeof window<"u"&&window.DocSign){let i=window.DocSign;i.isAuthenticated=Q,i.getCurrentUser=ce,i.getAccessToken=z,i.getDocumentsRemaining=Mt,i.register=Dt,i.login=Rt,i.logout=Ye,i.refreshToken=Ge,i.forgotPassword=It,i.resetPassword=Ot,i.authenticatedFetch=At,i.validatePassword=Ft,i.validateEmail=Nt,i.onAuthStateChange=Pt,d.debug("Auth module initialized on window.DocSign")}}var de=[{setup:"What's black, white, and red all over?",punchline:"A newspaper! (Though we prefer digital signatures.)"},{setup:"Why did the pen break up with the pencil?",punchline:"It found someone more permanent."},{setup:"What do you call a signature that tells jokes?",punchline:"A pun-dit!"},{setup:"Why was the ink feeling blue?",punchline:"Because it was running low on self-esteem."},{setup:"Knock knock... Who's there?... Ink...",punchline:"Ink who?... Ink you should sign this document!",delay:2e3},{setup:"Knock knock... Who's there?... Sign...",punchline:"Sign who?... Sign here, please!",delay:2e3},{setup:"Knock knock... Who's there?... Document...",punchline:"Document who?... Document wait, let's get this signed!",delay:2e3},{setup:"Why did the contract go to therapy?",punchline:"It had too many issues."},{setup:"What did one signature say to the other?",punchline:"You're looking sharp today!"},{setup:"Why do signatures make great friends?",punchline:"They're always there when you need them."},{setup:"What's a document's favorite music?",punchline:"Heavy metal... because of all the paperclips!"},{setup:"Why was the PDF so calm?",punchline:"Because it was well-formatted."},{setup:"Why did the e-signature cross the road?",punchline:"To get to the other side... of the document!"},{setup:"What do you call a lazy signature?",punchline:"A sign of the times."},{setup:"Why are digital signatures so reliable?",punchline:"They never lose their pen!"}],X=new Set;function Ve(){X.size>=de.length&&X.clear();let i;do i=Math.floor(Math.random()*de.length);while(X.has(i));return X.add(i),de[i]}function En(){X.clear()}var he=g("LoadingOverlay"),kn=1500,Cn=800,x=null,$=null,Ut=0,ue=null;function Tn(){let i=document.createElement("div");return i.id="loading-overlay",i.className="loading-overlay",i.innerHTML=`
    <div class="loading-content">
      <div class="loading-spinner"></div>
      <div class="loading-joke">
        <p class="joke-setup"></p>
        <p class="joke-punchline"></p>
      </div>
      <p class="loading-status">Loading...</p>
    </div>
  `,i}function Qe(){if(document.getElementById("loading-overlay")){x=document.getElementById("loading-overlay");return}if(x=Tn(),document.body.appendChild(x),!document.getElementById("loading-overlay-styles")){let i=document.createElement("style");i.id="loading-overlay-styles",i.textContent=Pn,document.head.appendChild(i)}he.debug("Loading overlay initialized")}function ge(i="Loading..."){if(x||Qe(),!x)return;Ut=Date.now(),$=Ve();let e=x.querySelector(".joke-setup"),t=x.querySelector(".joke-punchline"),n=x.querySelector(".loading-status");e&&(e.textContent=$.setup),t&&(t.textContent="",t.classList.remove("visible")),n&&(n.textContent=i),x.classList.add("visible");let r=$.delay??kn;ue=setTimeout(()=>{t&&$&&(t.textContent=$.punchline,t.classList.add("visible"))},r),he.debug("Showing loading overlay")}function Xe(){if(!x)return;ue&&(clearTimeout(ue),ue=null);let i=Date.now()-Ut,e=Math.max(0,Cn-i);setTimeout(()=>{x&&x.classList.remove("visible"),$=null,he.debug("Hiding loading overlay")},e)}function Ht(i){if(!x)return;let e=x.querySelector(".loading-status");e&&(e.textContent=i)}async function _t(i,e="Loading..."){ge(e);try{return await i()}finally{Xe()}}var Pn=`
.loading-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(255, 255, 255, 0.95);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
  opacity: 0;
  visibility: hidden;
  transition: opacity 0.3s ease, visibility 0.3s ease;
}

.loading-overlay.visible {
  opacity: 1;
  visibility: visible;
}

.loading-content {
  text-align: center;
  max-width: 500px;
  padding: 2rem;
}

.loading-spinner {
  width: 60px;
  height: 60px;
  margin: 0 auto 2rem;
  border: 4px solid #e0e0e0;
  border-top-color: #0056b3;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading-joke {
  margin-bottom: 1.5rem;
  min-height: 100px;
}

.joke-setup {
  font-size: 1.25rem;
  color: #333;
  margin-bottom: 1rem;
  line-height: 1.5;
}

.joke-punchline {
  font-size: 1.25rem;
  color: #0056b3;
  font-weight: 600;
  opacity: 0;
  transform: translateY(10px);
  transition: opacity 0.5s ease, transform 0.5s ease;
}

.joke-punchline.visible {
  opacity: 1;
  transform: translateY(0);
}

.loading-status {
  font-size: 1rem;
  color: #666;
  margin-top: 1rem;
}

/* Geriatric-friendly: larger text on smaller screens */
@media (max-width: 600px) {
  .joke-setup,
  .joke-punchline {
    font-size: 1.1rem;
  }

  .loading-content {
    padding: 1.5rem;
  }
}
`;function jt(){if(typeof window<"u"&&window.DocSign){let i=window.DocSign;i.showLoadingOverlay=ge,i.hideLoadingOverlay=Xe,i.updateLoadingStatus=Ht,i.withLoadingOverlay=_t,he.debug("Loading overlay added to window.DocSign")}}var C=g("DocSign");var Ln="https://docsign-worker.orlandodowntownhome.workers.dev/signatures/sync";function zt(){v.mark(y.NAMESPACE_INIT),vt(),Tt(),Bt(),Qe(),jt();let i=window.DOCSIGN_SYNC_ENDPOINT||Ln;if(re({syncEndpoint:i,minBackoffMs:1e3,maxBackoffMs:3e4,retryIntervalMs:3e4,maxRetries:10}),typeof window<"u"&&window.DocSign){let e=window.DocSign;e.TypedSignature=D,e.createTypedSignature=je,e.SIGNATURE_FONTS=O,e.MobileSignatureModal=F,e.isMobileDevice=ze,e.createSignatureModal=qe,e.SignatureModal=j,e.initSignatureModal=$e,e.SignatureCapture=N,e.SignatureCaptureModal=B,e.createSignatureCaptureModal=We,e.perf=v,e.PERF_MARKS=y,e.withTiming=ot,e.withLoading=st}v.mark(y.INTERACTIVE),v.isEnabled()&&v.logMetrics(),C.info("DocSign TypeScript initialized"),C.debug("PDF Preview Bridge available:",typeof k<"u"),C.debug("DocSign namespace available:",typeof window.DocSign<"u"),C.debug("LocalSessionManager available:",typeof S<"u"),C.debug("SyncManager available:",typeof A<"u"),C.debug("TypedSignature available:",typeof D<"u"),C.debug("MobileSignatureModal available:",typeof F<"u"),C.debug("SignatureCapture available:",typeof N<"u"),C.debug("SignatureCaptureModal available:",typeof B<"u"),C.debug("Auth module available:",typeof Q<"u"),C.debug("User authenticated:",Q()),C.debug("Loading overlay available:",typeof ge<"u")}document.readyState==="loading"?document.addEventListener("DOMContentLoaded",zt):zt();export{de as JOKES,S as LocalSessionManager,F as MobileSignatureModal,y as PERF_MARKS,k as PdfPreviewBridge,O as SIGNATURE_FONTS,f as SYNC_EVENTS,N as SignatureCapture,B as SignatureCaptureModal,j as SignatureModal,A as SyncManager,D as TypedSignature,At as authenticatedFetch,te as categorizeError,We as createSignatureCaptureModal,qe as createSignatureModal,je as createTypedSignature,be as createUserError,w as docSignPdfBridge,tn as domPointToPdf,en as domRectToPdf,ee as ensurePdfJsLoaded,It as forgotPassword,z as getAccessToken,ce as getCurrentUser,Mt as getDocumentsRemaining,Se as getFileTooLargeError,we as getOfflineError,on as getPageRenderInfo,Ve as getRandomJoke,ie as getSyncManager,xe as getUnsupportedFileError,ye as getUserFriendlyError,L as hideErrorModal,H as hideErrorToast,Xe as hideLoadingOverlay,Bt as initAuthNamespace,vt as initDocSignNamespace,jt as initLoadingNamespace,Qe as initLoadingOverlay,Tt as initLocalSessionNamespace,$e as initSignatureModal,re as initSyncManager,Q as isAuthenticated,ze as isMobileDevice,Xt as isPdfJsLoaded,b as localSessionManager,Rt as login,Ye as logout,Pt as onAuthStateChange,Re as onOnlineStatusChanged,Le as onSyncCompleted,Me as onSyncFailed,De as onSyncProgress,Pe as onSyncStarted,rn as pdfPointToDom,nn as pdfRectToDom,v as perf,Zt as previewBridge,Ge as refreshToken,Dt as register,En as resetJokeHistory,Ot as resetPassword,Ce as showConfirmDialog,Ee as showErrorModal,ke as showErrorToast,ge as showLoadingOverlay,Ht as updateLoadingStatus,Nt as validateEmail,Ft as validatePassword,st as withLoading,_t as withLoadingOverlay,ot as withTiming,Wt as withTimingSync};
//# sourceMappingURL=bundle.js.map
