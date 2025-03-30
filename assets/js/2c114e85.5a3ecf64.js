"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[99012],{43023:(e,n,o)=>{o.d(n,{R:()=>i,x:()=>c});var t=o(63696);const r={},s=t.createContext(r);function i(e){const n=t.useContext(s);return t.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function c(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:i(e.components),t.createElement(s.Provider,{value:n},e.children)}},76197:(e,n,o)=>{o.r(n),o.d(n,{assets:()=>a,contentTitle:()=>c,default:()=>p,frontMatter:()=>i,metadata:()=>t,toc:()=>l});const t=JSON.parse('{"id":"commands/project","title":"project","description":"The moon project  (or moon p) command will display all available information about a","source":"@site/docs/commands/project.mdx","sourceDirName":"commands","slug":"/commands/project","permalink":"/docs/commands/project","draft":false,"unlisted":false,"editUrl":"https://github.com/moonrepo/moon/tree/master/website/docs/commands/project.mdx","tags":[],"version":"current","frontMatter":{"title":"project"},"sidebar":"docs","previous":{"title":"from-turborepo","permalink":"/docs/commands/migrate/from-turborepo"},"next":{"title":"project-graph","permalink":"/docs/commands/project-graph"}}');var r=o(62540),s=o(43023);const i={title:"project"},c=void 0,a={},l=[{value:"Arguments",id:"arguments",level:3},{value:"Options",id:"options",level:3},{value:"Example output",id:"example-output",level:2},{value:"Configuration",id:"configuration",level:3}];function d(e){const n={a:"a",code:"code",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,s.R)(),...e.components};return(0,r.jsxs)(r.Fragment,{children:[(0,r.jsxs)(n.p,{children:["The ",(0,r.jsx)(n.code,{children:"moon project <name>"})," (or ",(0,r.jsx)(n.code,{children:"moon p"}),") command will display all available information about a\nproject that has been configured and exists within the graph. If a project does not exist, the\nprogram will return with a 1 exit code."]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{className:"language-shell",children:"$ moon project web\n"})}),"\n",(0,r.jsx)(n.h3,{id:"arguments",children:"Arguments"}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"<name>"})," - Name or alias of a project, as defined in ",(0,r.jsx)(n.a,{href:"../config/workspace#projects",children:(0,r.jsx)(n.code,{children:"projects"})}),"."]}),"\n"]}),"\n",(0,r.jsx)(n.h3,{id:"options",children:"Options"}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.code,{children:"--json"})," - Print the project and its configuration as JSON."]}),"\n"]}),"\n",(0,r.jsx)(n.h2,{id:"example-output",children:"Example output"}),"\n",(0,r.jsxs)(n.p,{children:["The following output is an example of what this command prints, using our very own\n",(0,r.jsx)(n.code,{children:"@moonrepo/runtime"})," package."]}),"\n",(0,r.jsx)(n.pre,{children:(0,r.jsx)(n.code,{children:"RUNTIME\n\nProject: runtime\nAlias: @moonrepo/runtime\nSource: packages/runtime\nRoot: ~/Projects/moon/packages/runtime\nPlatform: node\nLanguage: typescript\nStack: unknown\nType: library\n\nDEPENDS ON\n\n  - types (implicit, production)\n\nINHERITS FROM\n\n  - .moon/tasks/node.yml\n\nTASKS\n\nbuild:\n  \u203a packemon build --addFiles --addExports --declaration\nformat:\n  \u203a prettier --check --config ../../prettier.config.js --ignore-path ../../.prettierignore --no-error-on-unmatched-pattern .\nlint:\n  \u203a eslint --cache --cache-location ./.eslintcache --color --ext .js,.ts,.tsx --ignore-path ../../.eslintignore --exit-on-fatal-error --no-error-on-unmatched-pattern --report-unused-disable-directives .\nlint-fix:\n  \u203a eslint --cache --cache-location ./.eslintcache --color --ext .js,.ts,.tsx --ignore-path ../../.eslintignore --exit-on-fatal-error --no-error-on-unmatched-pattern --report-unused-disable-directives . --fix\ntest:\n  \u203a jest --cache --color --preset jest-preset-moon --passWithNoTests\ntypecheck:\n  \u203a tsc --build\n\nFILE GROUPS\n\nconfigs:\n  - packages/runtime/*.{js,json}\nsources:\n  - packages/runtime/src/**/*\n  - packages/runtime/types/**/*\ntests:\n  - packages/runtime/tests/**/*\n"})}),"\n",(0,r.jsx)(n.h3,{id:"configuration",children:"Configuration"}),"\n",(0,r.jsxs)(n.ul,{children:["\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.a,{href:"../config/workspace#projects",children:(0,r.jsx)(n.code,{children:"projects"})})," in ",(0,r.jsx)(n.code,{children:".moon/workspace.yml"})]}),"\n",(0,r.jsxs)(n.li,{children:[(0,r.jsx)(n.a,{href:"../config/project#project",children:(0,r.jsx)(n.code,{children:"project"})})," in ",(0,r.jsx)(n.code,{children:"moon.yml"})]}),"\n"]})]})}function p(e={}){const{wrapper:n}={...(0,s.R)(),...e.components};return n?(0,r.jsx)(n,{...e,children:(0,r.jsx)(d,{...e})}):d(e)}}}]);