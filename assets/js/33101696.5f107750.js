"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[11322],{43023:(e,n,t)=>{t.d(n,{R:()=>s,x:()=>l});var a=t(63696);const o={},r=a.createContext(o);function s(e){const n=a.useContext(r);return a.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function l(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(o):e.components||o:s(e.components),a.createElement(r.Provider,{value:n},e.children)}},54291:(e,n,t)=>{t.d(n,{A:()=>r});var a=t(59115),o=t(62540);function r(e){let{header:n,inline:t,updated:r,version:s}=e;return(0,o.jsx)(a.A,{text:`v${s}`,variant:r?"success":"info",className:n?"absolute right-0 top-1.5":t?"inline-block":"ml-2"})}},59115:(e,n,t)=>{t.d(n,{A:()=>l});var a=t(11750),o=t(20916),r=t(62540);const s={failure:"bg-red-100 text-red-900",info:"bg-pink-100 text-pink-900",success:"bg-green-100 text-green-900",warning:"bg-orange-100 text-orange-900"};function l(e){let{className:n,icon:t,text:l,variant:i}=e;return(0,r.jsxs)("span",{className:(0,a.A)("inline-flex items-center px-1 py-0.5 rounded text-xs font-bold uppercase",i?s[i]:"bg-gray-100 text-gray-800",n),children:[t&&(0,r.jsx)(o.A,{icon:t,className:"mr-1"}),l]})}},59300:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>c,contentTitle:()=>i,default:()=>h,frontMatter:()=>l,metadata:()=>a,toc:()=>d});const a=JSON.parse('{"id":"commands/task-graph","title":"task-graph","description":"The moon task-graph [target] (or moon tg) command will generate and serve a visual graph of all","source":"@site/docs/commands/task-graph.mdx","sourceDirName":"commands","slug":"/commands/task-graph","permalink":"/docs/commands/task-graph","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/commands/task-graph.mdx","tags":[],"version":"current","frontMatter":{"title":"task-graph"},"sidebar":"docs","previous":{"title":"task","permalink":"/docs/commands/task"},"next":{"title":"teardown","permalink":"/docs/commands/teardown"}}');var o=t(62540),r=t(43023),s=t(54291);const l={title:"task-graph"},i=void 0,c={},d=[{value:"Arguments",id:"arguments",level:3},{value:"Options",id:"options",level:3},{value:"Example output",id:"example-output",level:2}];function p(e){const n={a:"a",blockquote:"blockquote",code:"code",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,r.R)(),...e.components};return(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(s.A,{version:"1.30.0",header:!0}),"\n",(0,o.jsxs)(n.p,{children:["The ",(0,o.jsx)(n.code,{children:"moon task-graph [target]"})," (or ",(0,o.jsx)(n.code,{children:"moon tg"}),") command will generate and serve a visual graph of all\nconfigured tasks as nodes, with dependencies between as edges, and can also output the graph in\n",(0,o.jsx)(n.a,{href:"https://graphviz.org/doc/info/lang.html",children:"Graphviz DOT format"}),"."]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-shell",children:"# Run the visualizer locally\n$ moon task-graph\n\n# Export to DOT format\n$ moon task-graph --dot > graph.dot\n"})}),"\n",(0,o.jsxs)(n.blockquote,{children:["\n",(0,o.jsxs)(n.p,{children:["A task target can be passed to focus the graph to only that task and its dependencies. For\nexample, ",(0,o.jsx)(n.code,{children:"moon task-graph app:build"}),"."]}),"\n"]}),"\n",(0,o.jsx)(n.h3,{id:"arguments",children:"Arguments"}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"[target]"})," - Optional target of task to focus."]}),"\n"]}),"\n",(0,o.jsx)(n.h3,{id:"options",children:"Options"}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"--dependents"})," - Include direct dependents of the focused task."]}),"\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"--dot"})," - Print the graph in DOT format."]}),"\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"--json"})," - Print the graph in JSON format."]}),"\n"]}),"\n",(0,o.jsx)(n.h2,{id:"example-output",children:"Example output"}),"\n",(0,o.jsx)(n.p,{children:"The following output is an example of the graph in DOT format."}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-dot",children:'digraph {\n    0 [ label="types:build" style=filled, shape=oval, fillcolor=gray, fontcolor=black]\n    1 [ label="runtime:build" style=filled, shape=oval, fillcolor=gray, fontcolor=black]\n    2 [ label="website:build" style=filled, shape=oval, fillcolor=gray, fontcolor=black]\n    1 -> 0 [ label="required" arrowhead=box, arrowtail=box]\n    2 -> 1 [ label="required" arrowhead=box, arrowtail=box]\n    2 -> 0 [ label="required" arrowhead=box, arrowtail=box]\n}\n'})})]})}function h(e={}){const{wrapper:n}={...(0,r.R)(),...e.components};return n?(0,o.jsx)(n,{...e,children:(0,o.jsx)(p,{...e})}):p(e)}}}]);