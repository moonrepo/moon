"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[43071],{43023:(e,n,o)=>{o.d(n,{R:()=>s,x:()=>l});var r=o(63696);const t={},c=r.createContext(t);function s(e){const n=r.useContext(c);return r.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function l(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(t):e.components||t:s(e.components),r.createElement(c.Provider,{value:n},e.children)}},76099:(e,n,o)=>{o.r(n),o.d(n,{assets:()=>a,contentTitle:()=>l,default:()=>p,frontMatter:()=>s,metadata:()=>r,toc:()=>i});const r=JSON.parse('{"id":"commands/project-graph","title":"project-graph","description":"The moon project-graph [id] (or moon pg) command will generate and serve a visual graph of all","source":"@site/docs/commands/project-graph.mdx","sourceDirName":"commands","slug":"/commands/project-graph","permalink":"/docs/commands/project-graph","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/commands/project-graph.mdx","tags":[],"version":"current","frontMatter":{"title":"project-graph"},"sidebar":"docs","previous":{"title":"project","permalink":"/docs/commands/project"},"next":{"title":"query","permalink":"/docs/commands/query"}}');var t=o(62540),c=o(43023);const s={title:"project-graph"},l=void 0,a={},i=[{value:"Arguments",id:"arguments",level:3},{value:"Options",id:"options",level:3},{value:"Configuration",id:"configuration",level:3},{value:"Example output",id:"example-output",level:2}];function d(e){const n={a:"a",blockquote:"blockquote",code:"code",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,c.R)(),...e.components};return(0,t.jsxs)(t.Fragment,{children:[(0,t.jsxs)(n.p,{children:["The ",(0,t.jsx)(n.code,{children:"moon project-graph [id]"})," (or ",(0,t.jsx)(n.code,{children:"moon pg"}),") command will generate and serve a visual graph of all\nconfigured projects as nodes, with dependencies between as edges, and can also output the graph in\n",(0,t.jsx)(n.a,{href:"https://graphviz.org/doc/info/lang.html",children:"Graphviz DOT format"}),"."]}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-shell",children:"# Run the visualizer locally\n$ moon project-graph\n\n# Export to DOT format\n$ moon project-graph --dot > graph.dot\n"})}),"\n",(0,t.jsxs)(n.blockquote,{children:["\n",(0,t.jsxs)(n.p,{children:["A project name can be passed to focus the graph to only that project and its dependencies. For\nexample, ",(0,t.jsx)(n.code,{children:"moon project-graph app"}),"."]}),"\n"]}),"\n",(0,t.jsx)(n.h3,{id:"arguments",children:"Arguments"}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"[name]"})," - Optional name or alias of a project to focus, as defined in\n",(0,t.jsx)(n.a,{href:"../config/workspace#projects",children:(0,t.jsx)(n.code,{children:"projects"})}),"."]}),"\n"]}),"\n",(0,t.jsx)(n.h3,{id:"options",children:"Options"}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--dependents"})," - Include direct dependents of the focused project."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--dot"})," - Print the graph in DOT format."]}),"\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.code,{children:"--json"})," - Print the graph in JSON format."]}),"\n"]}),"\n",(0,t.jsx)(n.h3,{id:"configuration",children:"Configuration"}),"\n",(0,t.jsxs)(n.ul,{children:["\n",(0,t.jsxs)(n.li,{children:[(0,t.jsx)(n.a,{href:"../config/workspace#projects",children:(0,t.jsx)(n.code,{children:"projects"})})," in ",(0,t.jsx)(n.code,{children:".moon/workspace.yml"})]}),"\n"]}),"\n",(0,t.jsx)(n.h2,{id:"example-output",children:"Example output"}),"\n",(0,t.jsx)(n.p,{children:"The following output is an example of the graph in DOT format."}),"\n",(0,t.jsx)(n.pre,{children:(0,t.jsx)(n.code,{className:"language-dot",children:'digraph {\n    0 [ label="(workspace)" style=filled, shape=circle, fillcolor=black, fontcolor=white]\n    1 [ label="runtime" style=filled, shape=circle, fillcolor=gray, fontcolor=black]\n    2 [ label="website" style=filled, shape=circle, fillcolor=gray, fontcolor=black]\n    0 -> 1 [ arrowhead=none]\n    0 -> 2 [ arrowhead=none]\n}\n'})})]})}function p(e={}){const{wrapper:n}={...(0,c.R)(),...e.components};return n?(0,t.jsx)(n,{...e,children:(0,t.jsx)(d,{...e})}):d(e)}}}]);