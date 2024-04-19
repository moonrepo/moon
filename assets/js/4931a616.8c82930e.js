"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[48529],{17571:(e,t,n)=>{n.r(t),n.d(t,{assets:()=>c,contentTitle:()=>i,default:()=>h,frontMatter:()=>s,metadata:()=>l,toc:()=>d});var r=n(24246),o=n(71670),a=(n(33337),n(39798),n(76911));const s={slug:"v0.24",title:"moon v0.24 - Remote caching, interactive tasks, query improvements, and more",authors:["milesj","jpoz"],tags:["project","platform","moonbase","remote-cache"],image:"./img/v0.24.png"},i=void 0,l={permalink:"/blog/v0.24",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2023-02-13_v0.24.mdx",source:"@site/blog/2023-02-13_v0.24.mdx",title:"moon v0.24 - Remote caching, interactive tasks, query improvements, and more",description:"With this release, we've polished our CLI experience and improved task interoperability.",date:"2023-02-13T00:00:00.000Z",tags:[{label:"project",permalink:"/blog/tags/project"},{label:"platform",permalink:"/blog/tags/platform"},{label:"moonbase",permalink:"/blog/tags/moonbase"},{label:"remote-cache",permalink:"/blog/tags/remote-cache"}],readingTime:4.28,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"},{name:"James Pozdena",title:"Founder, developer",url:"https://github.com/jpoz",imageURL:"/img/authors/james.jpg",key:"jpoz"}],frontMatter:{slug:"v0.24",title:"moon v0.24 - Remote caching, interactive tasks, query improvements, and more",authors:["milesj","jpoz"],tags:["project","platform","moonbase","remote-cache"],image:"./img/v0.24.png"},unlisted:!1,prevItem:{title:"moon v0.25 - Deno tier 2 support, CI insights, custom project languages, and more",permalink:"/blog/v0.25"},nextItem:{title:"Remote caching is now publicly available through moonbase",permalink:"/blog/moonbase"}},c={image:n(35602).Z,authorsImageUrls:[void 0,void 0]},d=[{value:"Remote caching now available",id:"remote-caching-now-available",level:2},{value:"Project-level task platform",id:"project-level-task-platform",level:2},{value:"Interactive tasks",id:"interactive-tasks",level:2},{value:"Improved <code>moon query</code> commands (breaking)",id:"improved-moon-query-commands-breaking",level:2},{value:"New <code>moon query tasks</code> command",id:"new-moon-query-tasks-command",level:2},{value:"Shell completions",id:"shell-completions",level:2},{value:"Other changes",id:"other-changes",level:2},{value:"What&#39;s next?",id:"whats-next",level:2}];function u(e){const t={a:"a",blockquote:"blockquote",code:"code",em:"em",h2:"h2",li:"li",p:"p",pre:"pre",ul:"ul",...(0,o.a)(),...e.components};return(0,r.jsxs)(r.Fragment,{children:[(0,r.jsx)(t.p,{children:"With this release, we've polished our CLI experience and improved task interoperability."}),"\n",(0,r.jsx)(t.h2,{id:"remote-caching-now-available",children:"Remote caching now available"}),"\n",(0,r.jsxs)(t.p,{children:["If you missed our announcement earlier this week,\n",(0,r.jsx)(t.a,{href:"/blog/moonbase",children:"remote caching is now publicly available through our new service moonbase"}),"! If\nyou're looking to speed up your CI pipelines and share build artifacts between runs, moonbase can\nhelp."]}),"\n",(0,r.jsx)("div",{class:"flex justify-center",children:(0,r.jsx)(a.Z,{label:"Try moonbase for free!",href:"https://moonrepo.app",size:"lg"})}),"\n",(0,r.jsx)(t.h2,{id:"project-level-task-platform",children:"Project-level task platform"}),"\n",(0,r.jsxs)(t.p,{children:["In our previous release, ",(0,r.jsx)(t.a,{href:"./v0.23#project-level-environment-variables",children:"v0.23"}),", we added support for\nproject-level environment variables. This is great as it helps to reduce duplication for projects\nwith many tasks. On that note, we wondered which configuration field we could apply similar\ntreatment, and as such, we've added a project-level ",(0,r.jsx)(t.a,{href:"/docs/config/project#platform",children:(0,r.jsx)(t.code,{children:"platform"})}),"\nsetting."]}),"\n",(0,r.jsxs)(t.p,{children:["When this setting is defined, all task's within the current project that have ",(0,r.jsx)(t.em,{children:"not explicitly"}),"\nconfigured their ",(0,r.jsx)(t.code,{children:"platform"}),", will inherit the project-level platform. If neither settings are\ndefined, we'll attempt to detect the correct platform based on the state of the project."]}),"\n",(0,r.jsx)(t.pre,{children:(0,r.jsx)(t.code,{className:"language-yaml",metastring:'title="<project>/moon.yml"',children:"# Will set non-explicit task's platform to node\nplatform: 'node'\n\ntasks:\n  # Will be node\n  dev: # ...\n\n  # Will be node\n  build: # ...\n\n  # Will be system\n  serve:\n    # ...\n    platform: 'system'\n"})}),"\n",(0,r.jsxs)(t.blockquote,{children:["\n",(0,r.jsxs)(t.p,{children:["This setting is ",(0,r.jsx)(t.em,{children:"extremely useful"})," for projects that contain multiple languages. Even more so once\nwe land Bun and Deno support, as we'll need a way to differentiate JavaScript/TypeScript projects!"]}),"\n"]}),"\n",(0,r.jsx)(t.h2,{id:"interactive-tasks",children:"Interactive tasks"}),"\n",(0,r.jsxs)(t.p,{children:["When moon executes a task, it streams both stdout and stderr to the terminal ",(0,r.jsx)(t.em,{children:"and"})," captures the\noutput for later use. We do this for 2 reasons:"]}),"\n",(0,r.jsxs)(t.ul,{children:["\n",(0,r.jsx)(t.li,{children:"We store stdout.log and stderr.log files in a tarball archive."}),"\n",(0,r.jsx)(t.li,{children:"We replay this captured output when executing a task that has been cached."}),"\n"]}),"\n",(0,r.jsx)(t.p,{children:"While this works, our approach is non-standard. Streams are either piped or inherited, not both!\nBecause of our custom abstraction around streams and output capturing, it disrupts stdin, breaking\nall interactive commands. If you tried to run a task that prompted you with a question and were\nunable to answer it, this is why!"}),"\n",(0,r.jsxs)(t.p,{children:["To remedy this shortcoming, we're approaching this from 2 angles. The first is that all tasks marked\nas ",(0,r.jsx)(t.a,{href:"/docs/config/project#local",children:(0,r.jsx)(t.code,{children:"local"})})," (or have caching disabled) will no longer capture streamed\noutput, and will instead stream natively, allowing interactivity out of the box, but only when\nthey're the only task being ran. This will cover the majority of use cases."]}),"\n",(0,r.jsxs)(t.p,{children:["For the remaining use cases, we're introducing a new ",(0,r.jsx)(t.code,{children:"--interactive"})," flag for\n",(0,r.jsx)(t.a,{href:"/docs/commands/run",children:(0,r.jsx)(t.code,{children:"moon run"})}),". When this flag is provided, it will force the target into an\ninteractive mode."]}),"\n",(0,r.jsx)(t.pre,{children:(0,r.jsx)(t.code,{className:"language-shell",children:"$ moon run app:new --interactive\n"})}),"\n",(0,r.jsxs)(t.h2,{id:"improved-moon-query-commands-breaking",children:["Improved ",(0,r.jsx)(t.code,{children:"moon query"})," commands (breaking)"]}),"\n",(0,r.jsxs)(t.p,{children:["The ",(0,r.jsx)(t.a,{href:"/docs/commands/query/projects",children:(0,r.jsx)(t.code,{children:"moon query projects"})})," and\n",(0,r.jsx)(t.a,{href:"/docs/commands/query/touched-files",children:(0,r.jsx)(t.code,{children:"moon query touched-files"})})," commands are useful for building\ncustom solutions and integrations on top of moon, but they weren't developer friendly as they output\nlarge JSON blobs. To remedy this, we've updated both commands to output a simple human readable\nformat by default, and moved the JSON output behind a ",(0,r.jsx)(t.code,{children:"--json"})," flag."]}),"\n",(0,r.jsxs)(t.p,{children:["For example, ",(0,r.jsx)(t.code,{children:"moon query touched-files"})," now outputs a list of absolute file paths separated by new\nlines."]}),"\n",(0,r.jsx)(t.pre,{children:(0,r.jsx)(t.code,{children:"$ moon query touched-files\n/moon/website/docs/commands/query/projects.mdx\n/moon/crates/cli/tests/query_test.rs\n/moon/crates/cli/src/commands/query.rs\n/moon/website/blog/2023-02-13_v0.24.mdx\n"})}),"\n",(0,r.jsxs)(t.p,{children:["While ",(0,r.jsx)(t.code,{children:"moon query projects"})," now outputs a list of project separated by new lines, where each line\ncontains the project name, source, type, and language."]}),"\n",(0,r.jsx)(t.pre,{children:(0,r.jsx)(t.code,{children:"$ moon query projects\nreport | packages/report | library | typescript\nruntime | packages/runtime | library | typescript\ntypes | packages/types | library | typescript\nvisualizer | packages/visualizer | library | typescript\nwebsite | website | application | typescript\n"})}),"\n",(0,r.jsx)(t.p,{children:"We had 2 goals in mind for this change, the first was to make it easily readable, and the second was\nfor the default output to be easily parseable. We believe we've accomplished these goals!"}),"\n",(0,r.jsxs)(t.h2,{id:"new-moon-query-tasks-command",children:["New ",(0,r.jsx)(t.code,{children:"moon query tasks"})," command"]}),"\n",(0,r.jsxs)(t.p,{children:['To expand on the query improvements above, we wanted to provide a way to also query for tasks,\nanswering the question of "What tasks exists and for what projects?". And with this, we\'re\nintroducing a new ',(0,r.jsx)(t.a,{href:"/docs/commands/query/tasks",children:(0,r.jsx)(t.code,{children:"moon query tasks"})})," command!"]}),"\n",(0,r.jsx)(t.pre,{children:(0,r.jsx)(t.code,{children:"$ moon query tasks\ntypes\n  :build | packemon\n  :format | prettier\n  :lint | eslint\n  :test | jest\n  :typecheck | tsc\nreport\n  :build | packemon\n  :format | prettier\n  :lint | eslint\n  :test | jest\n  :typecheck | tsc\n...\n"})}),"\n",(0,r.jsx)(t.h2,{id:"shell-completions",children:"Shell completions"}),"\n",(0,r.jsxs)(t.p,{children:["Auto-completion in your terminal increases productivity, which we're a massive fan of. To help\nsupport this, we're introducing the ",(0,r.jsx)(t.a,{href:"/docs/commands/completions",children:(0,r.jsx)(t.code,{children:"moon completions"})})," command, which\ngenerates the appropriate command completions for your current shell."]}),"\n",(0,r.jsx)(t.p,{children:"This command writes to stdout, which can then be redirected to a file of your choice. Be sure to\nconfigure your shell profile to load the completions!"}),"\n",(0,r.jsx)(t.pre,{children:(0,r.jsx)(t.code,{className:"language-shell",children:"$ moon completions > ~/.bash_completion.d/moon.sh\n"})}),"\n",(0,r.jsx)(t.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,r.jsxs)(t.p,{children:["View the\n",(0,r.jsx)(t.a,{href:"https://github.com/moonrepo/moon/releases/tag/%40moonrepo%2Fcli%400.24.0",children:"official release"})," for a\nfull list of changes."]}),"\n",(0,r.jsxs)(t.ul,{children:["\n",(0,r.jsxs)(t.li,{children:["Added ",(0,r.jsx)(t.a,{href:"https://devblogs.microsoft.com/typescript/announcing-typescript-5-0-beta/",children:"TypeScript v5"}),"\nsupport."]}),"\n",(0,r.jsxs)(t.li,{children:["Added a ",(0,r.jsx)(t.code,{children:"hasher.walkStrategy"})," setting to ",(0,r.jsx)(t.code,{children:".moon/workspace.yml"}),"."]}),"\n",(0,r.jsxs)(t.li,{children:["Updated task ",(0,r.jsx)(t.code,{children:"outputs"})," to support token functions (",(0,r.jsx)(t.code,{children:"@group"}),", ",(0,r.jsx)(t.code,{children:"@globs"}),", etc)."]}),"\n",(0,r.jsx)(t.li,{children:"Reworked our comparison/baseline estimations calcuations."}),"\n"]}),"\n",(0,r.jsx)(t.h2,{id:"whats-next",children:"What's next?"}),"\n",(0,r.jsx)(t.p,{children:"Expect the following in the v0.25 release!"}),"\n",(0,r.jsxs)(t.ul,{children:["\n",(0,r.jsx)(t.li,{children:"Deno tier 2 support."}),"\n",(0,r.jsx)(t.li,{children:"CI insights and metrics within moonbase."}),"\n",(0,r.jsx)(t.li,{children:"Project-level TypeScript settings."}),"\n"]})]})}function h(e={}){const{wrapper:t}={...(0,o.a)(),...e.components};return t?(0,r.jsx)(t,{...e,children:(0,r.jsx)(u,{...e})}):u(e)}},39798:(e,t,n)=>{n.d(t,{Z:()=>s});n(27378);var r=n(40624);const o={tabItem:"tabItem_wHwb"};var a=n(24246);function s(e){let{children:t,hidden:n,className:s}=e;return(0,a.jsx)("div",{role:"tabpanel",className:(0,r.Z)(o.tabItem,s),hidden:n,children:t})}},33337:(e,t,n)=>{n.d(t,{Z:()=>p});var r=n(27378),o=n(40624),a=n(83457),s=n(35595),i=n(76457);const l={tabList:"tabList_J5MA",tabItem:"tabItem_l0OV"};var c=n(24246);function d(e){let{className:t,block:n,selectedValue:r,selectValue:s,tabValues:i}=e;const d=[],{blockElementScrollPositionUntilNextRender:u}=(0,a.o5)(),h=e=>{const t=e.currentTarget,n=d.indexOf(t),o=i[n].value;o!==r&&(u(t),s(o))},p=e=>{let t=null;switch(e.key){case"Enter":h(e);break;case"ArrowRight":{const n=d.indexOf(e.currentTarget)+1;t=d[n]??d[0];break}case"ArrowLeft":{const n=d.indexOf(e.currentTarget)-1;t=d[n]??d[d.length-1];break}}t?.focus()};return(0,c.jsx)("ul",{role:"tablist","aria-orientation":"horizontal",className:(0,o.Z)("tabs",{"tabs--block":n},t),children:i.map((e=>{let{value:t,label:n,attributes:a}=e;return(0,c.jsx)("li",{role:"tab",tabIndex:r===t?0:-1,"aria-selected":r===t,ref:e=>d.push(e),onKeyDown:p,onClick:h,...a,className:(0,o.Z)("tabs__item",l.tabItem,a?.className,{"tabs__item--active":r===t}),children:n??t},t)}))})}function u(e){let{lazy:t,children:n,selectedValue:o}=e;const a=(Array.isArray(n)?n:[n]).filter(Boolean);if(t){const e=a.find((e=>e.props.value===o));return e?(0,r.cloneElement)(e,{className:"margin-top--md"}):null}return(0,c.jsx)("div",{className:"margin-top--md",children:a.map(((e,t)=>(0,r.cloneElement)(e,{key:t,hidden:e.props.value!==o})))})}function h(e){const t=(0,s.Y)(e);return(0,c.jsxs)("div",{className:(0,o.Z)("tabs-container",l.tabList),children:[(0,c.jsx)(d,{...e,...t}),(0,c.jsx)(u,{...e,...t})]})}function p(e){const t=(0,i.Z)();return(0,c.jsx)(h,{...e,children:(0,s.h)(e.children)},String(t))}},35595:(e,t,n)=>{n.d(t,{Y:()=>p,h:()=>c});var r=n(27378),o=n(3620),a=n(9834),s=n(30654),i=n(70784),l=n(71819);function c(e){return r.Children.toArray(e).filter((e=>"\n"!==e)).map((e=>{if(!e||(0,r.isValidElement)(e)&&function(e){const{props:t}=e;return!!t&&"object"==typeof t&&"value"in t}(e))return e;throw new Error(`Docusaurus error: Bad <Tabs> child <${"string"==typeof e.type?e.type:e.type.name}>: all children of the <Tabs> component should be <TabItem>, and every <TabItem> should have a unique "value" prop.`)}))?.filter(Boolean)??[]}function d(e){const{values:t,children:n}=e;return(0,r.useMemo)((()=>{const e=t??function(e){return c(e).map((e=>{let{props:{value:t,label:n,attributes:r,default:o}}=e;return{value:t,label:n,attributes:r,default:o}}))}(n);return function(e){const t=(0,i.l)(e,((e,t)=>e.value===t.value));if(t.length>0)throw new Error(`Docusaurus error: Duplicate values "${t.map((e=>e.value)).join(", ")}" found in <Tabs>. Every value needs to be unique.`)}(e),e}),[t,n])}function u(e){let{value:t,tabValues:n}=e;return n.some((e=>e.value===t))}function h(e){let{queryString:t=!1,groupId:n}=e;const a=(0,o.k6)(),i=function(e){let{queryString:t=!1,groupId:n}=e;if("string"==typeof t)return t;if(!1===t)return null;if(!0===t&&!n)throw new Error('Docusaurus error: The <Tabs> component groupId prop is required if queryString=true, because this value is used as the search param name. You can also provide an explicit value such as queryString="my-search-param".');return n??null}({queryString:t,groupId:n});return[(0,s._X)(i),(0,r.useCallback)((e=>{if(!i)return;const t=new URLSearchParams(a.location.search);t.set(i,e),a.replace({...a.location,search:t.toString()})}),[i,a])]}function p(e){const{defaultValue:t,queryString:n=!1,groupId:o}=e,s=d(e),[i,c]=(0,r.useState)((()=>function(e){let{defaultValue:t,tabValues:n}=e;if(0===n.length)throw new Error("Docusaurus error: the <Tabs> component requires at least one <TabItem> children component");if(t){if(!u({value:t,tabValues:n}))throw new Error(`Docusaurus error: The <Tabs> has a defaultValue "${t}" but none of its children has the corresponding value. Available values are: ${n.map((e=>e.value)).join(", ")}. If you intend to show no default tab, use defaultValue={null} instead.`);return t}const r=n.find((e=>e.default))??n[0];if(!r)throw new Error("Unexpected error: 0 tabValues");return r.value}({defaultValue:t,tabValues:s}))),[p,m]=h({queryString:n,groupId:o}),[f,b]=function(e){let{groupId:t}=e;const n=function(e){return e?`docusaurus.tab.${e}`:null}(t),[o,a]=(0,l.Nk)(n);return[o,(0,r.useCallback)((e=>{n&&a.set(e)}),[n,a])]}({groupId:o}),g=(()=>{const e=p??f;return u({value:e,tabValues:s})?e:null})();(0,a.Z)((()=>{g&&c(g)}),[g]);return{selectedValue:i,selectValue:(0,r.useCallback)((e=>{if(!u({value:e,tabValues:s}))throw new Error(`Can't select invalid tab value=${e}`);c(e),m(e),b(e)}),[m,b,s]),tabValues:s}}},35602:(e,t,n)=>{n.d(t,{Z:()=>r});const r=n.p+"assets/images/v0.24-0e225eaeb8b3c60cc26907770c589000.png"},71670:(e,t,n)=>{n.d(t,{Z:()=>i,a:()=>s});var r=n(27378);const o={},a=r.createContext(o);function s(e){const t=r.useContext(a);return r.useMemo((function(){return"function"==typeof e?e(t):{...t,...e}}),[t,e])}function i(e){let t;return t=e.disableParentContext?"function"==typeof e.components?e.components(o):e.components||o:s(e.components),r.createElement(a.Provider,{value:t},e.children)}}}]);