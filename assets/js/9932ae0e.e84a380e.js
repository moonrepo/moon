"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[45129],{42588:e=>{e.exports=JSON.parse('{"permalink":"/blog/proto-v0.26-rc","editUrl":"https://github.com/moonrepo/moon/tree/master/website/blog/2023-12-19_proto-v0.26-rc.mdx","source":"@site/blog/2023-12-19_proto-v0.26-rc.mdx","title":"proto v0.26 (rc) - Release candidate available for testing!","description":"We\'ve got a very special release candidate that we\'d love to stress test before an official release!","date":"2023-12-19T00:00:00.000Z","tags":[{"inline":true,"label":"proto","permalink":"/blog/tags/proto"},{"inline":true,"label":"shim","permalink":"/blog/tags/shim"}],"readingTime":2.9,"hasTruncateMarker":true,"authors":[{"name":"Miles Johnson","title":"Founder, developer","url":"https://github.com/milesj","imageURL":"/img/authors/miles.jpg","key":"milesj","page":null}],"frontMatter":{"slug":"proto-v0.26-rc","title":"proto v0.26 (rc) - Release candidate available for testing!","authors":["milesj"],"tags":["proto","shim"]},"unlisted":false,"prevItem":{"title":"proto v0.26 - New native shim implementation","permalink":"/blog/proto-v0.26"},"nextItem":{"title":"moon v1.18 - New task execution flow and custom project names","permalink":"/blog/moon-v1.18"}}')},43023:(e,t,n)=>{n.d(t,{R:()=>r,x:()=>a});var o=n(63696);const s={},i=o.createContext(s);function r(e){const t=o.useContext(i);return o.useMemo((function(){return"function"==typeof e?e(t):{...t,...e}}),[t,e])}function a(e){let t;return t=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:r(e.components),o.createElement(i.Provider,{value:t},e.children)}},75340:(e,t,n)=>{n.r(t),n.d(t,{assets:()=>l,contentTitle:()=>a,default:()=>h,frontMatter:()=>r,metadata:()=>o,toc:()=>d});var o=n(42588),s=n(62540),i=n(43023);const r={slug:"proto-v0.26-rc",title:"proto v0.26 (rc) - Release candidate available for testing!",authors:["milesj"],tags:["proto","shim"]},a=void 0,l={authorsImageUrls:[void 0]},d=[{value:"What didn&#39;t work?",id:"what-didnt-work",level:2},{value:"What&#39;s new?",id:"whats-new",level:2},{value:"How to test?",id:"how-to-test",level:2},{value:"What to test?",id:"what-to-test",level:2}];function c(e){const t={a:"a",admonition:"admonition",code:"code",em:"em",h2:"h2",li:"li",p:"p",ul:"ul",...(0,i.R)(),...e.components};return(0,s.jsxs)(s.Fragment,{children:[(0,s.jsx)(t.p,{children:"We've got a very special release candidate that we'd love to stress test before an official release!"}),"\n",(0,s.jsx)(t.p,{children:"proto at its core is a version manager, which means like most version managers, it relies on a\nconcept known as shims. Shims are lightweight executable scripts that act like a proxy to the\nunderlying binary, and are useful for proto to intercept executions and inject custom functionality,\nlike our dynamic version detection."}),"\n",(0,s.jsxs)(t.p,{children:["On Unix machines, we relied on Bash scripts for shims, which worked rather well. However, on\nWindows, we relied on PowerShell scripts (",(0,s.jsx)(t.code,{children:".ps1"}),"), batch/cmd scripts (",(0,s.jsx)(t.code,{children:".cmd"}),"), and Bash scripts, all\nwith differing levels of functionality, and each serving a separate purpose. Windows support ",(0,s.jsx)(t.em,{children:"did\nnot"})," work well."]}),"\n",(0,s.jsx)(t.h2,{id:"what-didnt-work",children:"What didn't work?"}),"\n",(0,s.jsxs)(t.p,{children:["When using shims, you must ensure that all the following scenarios work well: piping data/commands,\nredirection, stdin prompts, interactivity, signal handling, exit code bubbling, so on and so forth.\nBash solves a lot of this for us, but Windows does not have a native Bash shell, and thus we had to\nrely on other scripting languages. The ",(0,s.jsx)(t.code,{children:".cmd"})," files barely supported any of this, and the ",(0,s.jsx)(t.code,{children:".ps1"}),"\nfiles were a bit better, but still not great."]}),"\n",(0,s.jsxs)(t.p,{children:["For the most part, executing a shim as-is and doing basic work was fine, but once you needed a\ncomplex scenario (like above), it broke down pretty quickly. It was also further exacerbated when\ndealing with nested shim executions, for example, ",(0,s.jsx)(t.code,{children:"npm"})," calls ",(0,s.jsx)(t.code,{children:"node"})," under the hood. The parent shim\nmay be executed with ",(0,s.jsx)(t.code,{children:".ps1"})," but the child may be ",(0,s.jsx)(t.code,{children:".cmd"}),", and these do not play well together."]}),"\n",(0,s.jsxs)(t.p,{children:["The other problem on Windows is that scripts are not true executables, and are not easily located on\n",(0,s.jsx)(t.code,{children:"PATH"})," (excluding ",(0,s.jsx)(t.code,{children:".cmd"})," files)."]}),"\n",(0,s.jsx)(t.h2,{id:"whats-new",children:"What's new?"}),"\n",(0,s.jsxs)(t.p,{children:["To combat all of these problems, we needed a truly native solution, and that's exactly what we did.\nWe wrote our own Rust based executable, that will replace all of the custom shim scripts, and can\nproperly handle all of the required scenarios. This new executable is named ",(0,s.jsx)(t.code,{children:"proto-shim"}),"\n(",(0,s.jsx)(t.code,{children:"proto-shim.exe"})," on Windows) and is published alongside the ",(0,s.jsx)(t.code,{children:"proto"})," binary."]}),"\n",(0,s.jsx)(t.p,{children:"This new executable solves all of the following problems (hopefully):"}),"\n",(0,s.jsxs)(t.ul,{children:["\n",(0,s.jsxs)(t.li,{children:["Locatable on ",(0,s.jsx)(t.code,{children:"PATH"})," (is an ",(0,s.jsx)(t.code,{children:".exe"})," for Windows)"]}),"\n",(0,s.jsx)(t.li,{children:"Can pipe/redirect data"}),"\n",(0,s.jsx)(t.li,{children:"Handles stdin prompts/interactivity"}),"\n",(0,s.jsx)(t.li,{children:"Supports ctrl+c interruptions"}),"\n",(0,s.jsx)(t.li,{children:"Passes parent signals to child processes"}),"\n",(0,s.jsx)(t.li,{children:"Attempts to kill child processes on parent exit"}),"\n",(0,s.jsx)(t.li,{children:"Bubbles exit codes"}),"\n",(0,s.jsx)(t.li,{children:"Native performance"}),"\n",(0,s.jsx)(t.li,{children:"Doesn't require special privileges (no symlinks)"}),"\n"]}),"\n",(0,s.jsx)(t.h2,{id:"how-to-test",children:"How to test?"}),"\n",(0,s.jsxs)(t.p,{children:["If you're interested in testing this new implementation (we'd appreciate it), you can do so by\ndownloading the latest release candidate from GitHub: ",(0,s.jsx)(t.a,{href:"https://github.com/moonrepo/proto/releases",children:"https://github.com/moonrepo/proto/releases"})]}),"\n",(0,s.jsxs)(t.p,{children:["Once downloaded, unpack the archive, and move the ",(0,s.jsx)(t.code,{children:"proto"})," and ",(0,s.jsx)(t.code,{children:"proto-shim"})," binaries to the\n",(0,s.jsx)(t.code,{children:"~/.proto/bin"})," directory (or the location of your ",(0,s.jsx)(t.code,{children:"PROTO_INSTALL_DIR"})," environment variable). From\nhere, you can execute ",(0,s.jsx)(t.code,{children:"proto"})," or your tool binaries as normal."]}),"\n",(0,s.jsx)(t.admonition,{type:"warning",children:(0,s.jsxs)(t.p,{children:["If you run into issues, try deleting the old ",(0,s.jsx)(t.code,{children:"~/.proto/shims"})," directory and trying again. If\nproblems still persist, please report an issue or reach out to us on Discord!"]})}),"\n",(0,s.jsx)(t.admonition,{type:"info",children:(0,s.jsxs)(t.p,{children:["On macOS, you may need to add the binary to the trusted list, in your Priacy & Security settings.\nThis can also be achieved on the command line with ",(0,s.jsx)(t.code,{children:"xattr -c ~/.proto/bin/*"}),"."]})}),"\n",(0,s.jsx)(t.h2,{id:"what-to-test",children:"What to test?"}),"\n",(0,s.jsxs)(t.p,{children:["Basically everything. We want to ensure that all of the functionality in ",(0,s.jsx)(t.a,{href:"#whats-new",children:"What's new?"}),"\nworks as expected, so simply go about your day to day development and let us know if you run into\nany issues!"]})]})}function h(e={}){const{wrapper:t}={...(0,i.R)(),...e.components};return t?(0,s.jsx)(t,{...e,children:(0,s.jsx)(c,{...e})}):c(e)}}}]);