"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[63165],{4386:(e,n,o)=>{o.r(n),o.d(n,{assets:()=>a,contentTitle:()=>r,default:()=>c,frontMatter:()=>i,metadata:()=>l,toc:()=>d});var s=o(24246),t=o(71670);const i={slug:"moon-v1.21",title:"moon v1.21 - Deno tier 3, file group improvements, task shells, and more!",authors:["milesj"],tags:["deno","file-groups","env","shell","tasks","turborepo"],image:"./img/moon/v1.21.png"},r=void 0,l={permalink:"/blog/moon-v1.21",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2024-02-07_moon-v1.21.mdx",source:"@site/blog/2024-02-07_moon-v1.21.mdx",title:"moon v1.21 - Deno tier 3, file group improvements, task shells, and more!",description:"With this release, get ready for Deno tier 3 support, file group and task improvements, a new",date:"2024-02-07T00:00:00.000Z",tags:[{label:"deno",permalink:"/blog/tags/deno"},{label:"file-groups",permalink:"/blog/tags/file-groups"},{label:"env",permalink:"/blog/tags/env"},{label:"shell",permalink:"/blog/tags/shell"},{label:"tasks",permalink:"/blog/tags/tasks"},{label:"turborepo",permalink:"/blog/tags/turborepo"}],readingTime:3.34,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"moon-v1.21",title:"moon v1.21 - Deno tier 3, file group improvements, task shells, and more!",authors:["milesj"],tags:["deno","file-groups","env","shell","tasks","turborepo"],image:"./img/moon/v1.21.png"},unlisted:!1,prevItem:{title:"moon v1.22 - Organizational settings, wildcard env var inputs, and Nx migration",permalink:"/blog/moon-v1.22"},nextItem:{title:"moon v1.20 - Extension plugins, default task options, and more",permalink:"/blog/moon-v1.20"}},a={image:o(47609).Z,authorsImageUrls:[void 0]},d=[{value:"Deno tier 3 support",id:"deno-tier-3-support",level:2},{value:"File groups now support environment variables",id:"file-groups-now-support-environment-variables",level:2},{value:"New <code>unixShell</code> and <code>windowsShell</code> task options",id:"new-unixshell-and-windowsshell-task-options",level:2},{value:"New <code>migrate-turborepo</code> extension",id:"new-migrate-turborepo-extension",level:2},{value:"Other changes",id:"other-changes",level:2}];function h(e){const n={a:"a",admonition:"admonition",code:"code",em:"em",h2:"h2",li:"li",p:"p",pre:"pre",ul:"ul",...(0,t.a)(),...e.components};return(0,s.jsxs)(s.Fragment,{children:[(0,s.jsx)(n.p,{children:"With this release, get ready for Deno tier 3 support, file group and task improvements, a new\nextension, and more."}),"\n",(0,s.jsx)(n.h2,{id:"deno-tier-3-support",children:"Deno tier 3 support"}),"\n",(0,s.jsxs)(n.p,{children:["We've supported Deno tier 1 and 2 for almost a year now, but were hesitant to support tier 3 until\n",(0,s.jsx)(n.a,{href:"/proto",children:"proto"})," stabilizes further. Now that proto is ",(0,s.jsx)(n.em,{children:"almost"})," at an official v1 release, and other\ntools in the toolchain (like Node.js, Bun, and Rust) are powered by proto, we're confident in\nsupporting Deno tier 3. To make use of this, simply set the\n",(0,s.jsx)(n.a,{href:"/docs/config/toolchain#deno",children:(0,s.jsx)(n.code,{children:"deno.version"})})," setting in\n",(0,s.jsx)(n.a,{href:"/docs/config/toolchain",children:(0,s.jsx)(n.code,{children:".moon/toolchain.yml"})}),"."]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/toolchain.yml"',children:"deno:\n  version: '1.40.0'\n"})}),"\n",(0,s.jsx)(n.p,{children:"When enabled, moon will download and install that version of Deno in the background, and run all\nsubsequent tasks with it. This is great for ensuring that your project is always using the same\nversion of Deno, across all machines."}),"\n",(0,s.jsx)(n.h2,{id:"file-groups-now-support-environment-variables",children:"File groups now support environment variables"}),"\n",(0,s.jsxs)(n.p,{children:["Task ",(0,s.jsx)(n.a,{href:"/docs/config/project#inputs",children:(0,s.jsx)(n.code,{children:"inputs"})})," have supported environment variables for a while now,\nbut file groups have not. The main reason for this is that file groups were implemented far before\nenvironment variables in task inputs! To bridge this gap, we've added support for environment\nvariables in file groups."]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",children:"fileGroups:\n  vite:\n    - '...'\n    - '$VITE_SECRET_KEY'\n    - '$NODE_ENV'\n\ntasks:\n  build:\n    command: 'vite build'\n    inputs:\n      - '@group(vite)'\n"})}),"\n",(0,s.jsxs)(n.p,{children:["Environment variables can be referenced using the\n",(0,s.jsxs)(n.a,{href:"/docs/concepts/token#group",children:[(0,s.jsx)(n.code,{children:"@group"})," token function"]}),", or the new\n",(0,s.jsxs)(n.a,{href:"/docs/concepts/token#envs",children:[(0,s.jsx)(n.code,{children:"@envs"})," token function"]}),". The latter is only supported for ",(0,s.jsx)(n.code,{children:"inputs"})," and\nwill error for other locations, while the former is supported in ",(0,s.jsx)(n.code,{children:"args"}),", ",(0,s.jsx)(n.code,{children:"inputs"}),", and ",(0,s.jsx)(n.code,{children:"outputs"}),",\nbut will filter out environment variables when they are not supported."]}),"\n",(0,s.jsxs)(n.h2,{id:"new-unixshell-and-windowsshell-task-options",children:["New ",(0,s.jsx)(n.code,{children:"unixShell"})," and ",(0,s.jsx)(n.code,{children:"windowsShell"})," task options"]}),"\n",(0,s.jsxs)(n.p,{children:["When the ",(0,s.jsx)(n.a,{href:"/docs/config/project#shell",children:(0,s.jsx)(n.code,{children:"shell"})})," task option is enabled, we run the task within a\nshell. However, the chosen shell was hard-coded to ",(0,s.jsx)(n.code,{children:"$SHELL"})," on Unix machines and PowerShell on\nWindows, but what if you wanted to run it with a different shell? Or the same shell across all\noperating systems? Well, you couldn't."]}),"\n",(0,s.jsxs)(n.p,{children:["But not anymore! With this release, we're introducing ",(0,s.jsx)(n.a,{href:"/docs/config/project#unixshell",children:(0,s.jsx)(n.code,{children:"unixShell"})}),"\nand ",(0,s.jsx)(n.a,{href:"/docs/config/project#windowsshell",children:(0,s.jsx)(n.code,{children:"windowsShell"})})," task options. When paired with ",(0,s.jsx)(n.code,{children:"shell"}),", the\ntask will run in a shell of your choice. For example, why not Bash everywhere?"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  build:\n    command: 'vite build'\n    options:\n      shell: true\n      unixShell: 'bash'\n      windowsShell: 'bash'\n"})}),"\n",(0,s.jsxs)(n.h2,{id:"new-migrate-turborepo-extension",children:["New ",(0,s.jsx)(n.code,{children:"migrate-turborepo"})," extension"]}),"\n",(0,s.jsxs)(n.p,{children:["In our previous release, we added support for ",(0,s.jsx)(n.a,{href:"./moon-v1.20",children:"extensions, a new kind of WASM plugin"}),".\nSince this is a new experimental feature, we really wanted to show off what it can do, and stress\ntest its boundaries. To do that, we chose to migrate the old ",(0,s.jsx)(n.code,{children:"moon migrate from-turborepo"})," command\ninto an extension\n(",(0,s.jsx)(n.a,{href:"https://github.com/moonrepo/moon-extensions/tree/master/crates/migrate-turborepo",children:"source can be found here"}),").\nThis is our most complex extension so far, as it:"]}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsx)(n.li,{children:"Loads and parses files on the file system."}),"\n",(0,s.jsx)(n.li,{children:"Reads and writes JSON and YAML files."}),"\n",(0,s.jsx)(n.li,{children:"Supports deserializing data into structs."}),"\n",(0,s.jsxs)(n.li,{children:["Extracts project graph information by executing ",(0,s.jsx)(n.code,{children:"moon project-graph"}),"."]}),"\n"]}),"\n",(0,s.jsxs)(n.p,{children:["Do you currently have a Turborepo powered repository? And want to migrate to moon? Then simply\nexecute the extension as such. View our\n",(0,s.jsx)(n.a,{href:"/docs/guides/extensions#migrate-turborepo",children:"guide for more information"}),"!"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-shell",children:"$ moon ext migrate-turborepo\n"})}),"\n",(0,s.jsx)(n.p,{children:"As part of the migration from moon's Rust core into a WASM plugin, we've added support for the\nfollowing new features:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["Added Bun support behind a new ",(0,s.jsx)(n.code,{children:"--bun"})," flag."]}),"\n",(0,s.jsxs)(n.li,{children:["Added support for Turbo's ",(0,s.jsx)(n.code,{children:"globalDotEnv"}),", ",(0,s.jsx)(n.code,{children:"dotEnv"}),", and ",(0,s.jsx)(n.code,{children:"outputMode"}),"."]}),"\n",(0,s.jsxs)(n.li,{children:["Added support for root-level tasks (",(0,s.jsx)(n.code,{children:"//#"}),") through a root ",(0,s.jsx)(n.code,{children:"moon.yml"}),", instead of logging a\nwarning."]}),"\n",(0,s.jsxs)(n.li,{children:["Updated migrated task commands to run through a package manager, instead of\n",(0,s.jsx)(n.code,{children:"moon node run-script"}),"."]}),"\n"]}),"\n",(0,s.jsx)(n.admonition,{type:"info",children:(0,s.jsxs)(n.p,{children:["Based on the success of this extension, we plan to support a ",(0,s.jsx)(n.code,{children:"migrate-nx"})," extension in the future!\nIf you'd like to help in this endeavor, let us know!"]})}),"\n",(0,s.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,s.jsxs)(n.p,{children:["View the ",(0,s.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/v1.21.0",children:"official release"})," for a full list\nof changes."]}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["Added ",(0,s.jsx)(n.code,{children:"bun.inferTasksFromScripts"})," setting to ",(0,s.jsx)(n.code,{children:".moon/toolchain.yml"}),", for compatibility with\nNode.js."]}),"\n",(0,s.jsxs)(n.li,{children:["Added a ",(0,s.jsx)(n.code,{children:"--quiet"})," global argument, for hiding non-critical moon output."]}),"\n",(0,s.jsxs)(n.li,{children:["Updated tasks with glob-like arguments to automatically enabled the ",(0,s.jsx)(n.code,{children:"shell"})," option, so that glob\nexpansion works correctly."]}),"\n",(0,s.jsx)(n.li,{children:"Implemented a new buffered console layer for writing to stdout/stderr."}),"\n"]})]})}function c(e={}){const{wrapper:n}={...(0,t.a)(),...e.components};return n?(0,s.jsx)(n,{...e,children:(0,s.jsx)(h,{...e})}):h(e)}},47609:(e,n,o)=>{o.d(n,{Z:()=>s});const s=o.p+"assets/images/v1.21-629fcdc1b47073ecaa5ce648b2b53d14.png"},71670:(e,n,o)=>{o.d(n,{Z:()=>l,a:()=>r});var s=o(27378);const t={},i=s.createContext(t);function r(e){const n=s.useContext(i);return s.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function l(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(t):e.components||t:r(e.components),s.createElement(i.Provider,{value:n},e.children)}}}]);