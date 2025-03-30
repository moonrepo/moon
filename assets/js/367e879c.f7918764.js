"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[15571],{43023:(e,n,t)=>{t.d(n,{R:()=>o,x:()=>i});var a=t(63696);const s={},r=a.createContext(s);function o(e){const n=a.useContext(r);return a.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function i(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(s):e.components||s:o(e.components),a.createElement(r.Provider,{value:n},e.children)}},65457:(e,n,t)=>{t.d(n,{A:()=>v});var a=t(63696),s=t(11750),r=t(93707),o=t(49519),i=t(83604),l=t(95196),c=t(76229),d=t(88030);function u(e){return a.Children.toArray(e).filter((e=>"\n"!==e)).map((e=>{if(!e||(0,a.isValidElement)(e)&&function(e){const{props:n}=e;return!!n&&"object"==typeof n&&"value"in n}(e))return e;throw new Error(`Docusaurus error: Bad <Tabs> child <${"string"==typeof e.type?e.type:e.type.name}>: all children of the <Tabs> component should be <TabItem>, and every <TabItem> should have a unique "value" prop.`)}))?.filter(Boolean)??[]}function h(e){const{values:n,children:t}=e;return(0,a.useMemo)((()=>{const e=n??function(e){return u(e).map((e=>{let{props:{value:n,label:t,attributes:a,default:s}}=e;return{value:n,label:t,attributes:a,default:s}}))}(t);return function(e){const n=(0,c.XI)(e,((e,n)=>e.value===n.value));if(n.length>0)throw new Error(`Docusaurus error: Duplicate values "${n.map((e=>e.value)).join(", ")}" found in <Tabs>. Every value needs to be unique.`)}(e),e}),[n,t])}function p(e){let{value:n,tabValues:t}=e;return t.some((e=>e.value===n))}function m(e){let{queryString:n=!1,groupId:t}=e;const s=(0,o.W6)(),r=function(e){let{queryString:n=!1,groupId:t}=e;if("string"==typeof n)return n;if(!1===n)return null;if(!0===n&&!t)throw new Error('Docusaurus error: The <Tabs> component groupId prop is required if queryString=true, because this value is used as the search param name. You can also provide an explicit value such as queryString="my-search-param".');return t??null}({queryString:n,groupId:t});return[(0,l.aZ)(r),(0,a.useCallback)((e=>{if(!r)return;const n=new URLSearchParams(s.location.search);n.set(r,e),s.replace({...s.location,search:n.toString()})}),[r,s])]}function f(e){const{defaultValue:n,queryString:t=!1,groupId:s}=e,r=h(e),[o,l]=(0,a.useState)((()=>function(e){let{defaultValue:n,tabValues:t}=e;if(0===t.length)throw new Error("Docusaurus error: the <Tabs> component requires at least one <TabItem> children component");if(n){if(!p({value:n,tabValues:t}))throw new Error(`Docusaurus error: The <Tabs> has a defaultValue "${n}" but none of its children has the corresponding value. Available values are: ${t.map((e=>e.value)).join(", ")}. If you intend to show no default tab, use defaultValue={null} instead.`);return n}const a=t.find((e=>e.default))??t[0];if(!a)throw new Error("Unexpected error: 0 tabValues");return a.value}({defaultValue:n,tabValues:r}))),[c,u]=m({queryString:t,groupId:s}),[f,g]=function(e){let{groupId:n}=e;const t=function(e){return e?`docusaurus.tab.${e}`:null}(n),[s,r]=(0,d.Dv)(t);return[s,(0,a.useCallback)((e=>{t&&r.set(e)}),[t,r])]}({groupId:s}),b=(()=>{const e=c??f;return p({value:e,tabValues:r})?e:null})();(0,i.A)((()=>{b&&l(b)}),[b]);return{selectedValue:o,selectValue:(0,a.useCallback)((e=>{if(!p({value:e,tabValues:r}))throw new Error(`Can't select invalid tab value=${e}`);l(e),u(e),g(e)}),[u,g,r]),tabValues:r}}var g=t(95200);const b={tabList:"tabList_J5MA",tabItem:"tabItem_l0OV"};var x=t(62540);function j(e){let{className:n,block:t,selectedValue:a,selectValue:o,tabValues:i}=e;const l=[],{blockElementScrollPositionUntilNextRender:c}=(0,r.a_)(),d=e=>{const n=e.currentTarget,t=l.indexOf(n),s=i[t].value;s!==a&&(c(n),o(s))},u=e=>{let n=null;switch(e.key){case"Enter":d(e);break;case"ArrowRight":{const t=l.indexOf(e.currentTarget)+1;n=l[t]??l[0];break}case"ArrowLeft":{const t=l.indexOf(e.currentTarget)-1;n=l[t]??l[l.length-1];break}}n?.focus()};return(0,x.jsx)("ul",{role:"tablist","aria-orientation":"horizontal",className:(0,s.A)("tabs",{"tabs--block":t},n),children:i.map((e=>{let{value:n,label:t,attributes:r}=e;return(0,x.jsx)("li",{role:"tab",tabIndex:a===n?0:-1,"aria-selected":a===n,ref:e=>{l.push(e)},onKeyDown:u,onClick:d,...r,className:(0,s.A)("tabs__item",b.tabItem,r?.className,{"tabs__item--active":a===n}),children:t??n},n)}))})}function k(e){let{lazy:n,children:t,selectedValue:r}=e;const o=(Array.isArray(t)?t:[t]).filter(Boolean);if(n){const e=o.find((e=>e.props.value===r));return e?(0,a.cloneElement)(e,{className:(0,s.A)("margin-top--md",e.props.className)}):null}return(0,x.jsx)("div",{className:"margin-top--md",children:o.map(((e,n)=>(0,a.cloneElement)(e,{key:n,hidden:e.props.value!==r})))})}function w(e){const n=f(e);return(0,x.jsxs)("div",{className:(0,s.A)("tabs-container",b.tabList),children:[(0,x.jsx)(j,{...n,...e}),(0,x.jsx)(k,{...n,...e})]})}function v(e){const n=(0,g.A)();return(0,x.jsx)(w,{...e,children:u(e.children)},String(n))}},69386:(e,n,t)=>{t.d(n,{A:()=>a});const a=t.p+"assets/images/v1.29-e00b51e586685d72b0ea1d5ba6c30b29.png"},92706:e=>{e.exports=JSON.parse('{"permalink":"/blog/moon-v1.29","editUrl":"https://github.com/moonrepo/moon/tree/master/website/blog/2024-10-07_moon-v1.29.mdx","source":"@site/blog/2024-10-07_moon-v1.29.mdx","title":"moon v1.29 - Improved affected tracking, experimental Pkl configuration, and more","description":"In this release, we\'re excited to introduce an improved affected tracker and a new (but","date":"2024-10-07T00:00:00.000Z","tags":[{"inline":true,"label":"affected","permalink":"/blog/tags/affected"},{"inline":true,"label":"detection","permalink":"/blog/tags/detection"},{"inline":true,"label":"tracker","permalink":"/blog/tags/tracker"},{"inline":true,"label":"project","permalink":"/blog/tags/project"},{"inline":true,"label":"task","permalink":"/blog/tags/task"},{"inline":true,"label":"config","permalink":"/blog/tags/config"},{"inline":true,"label":"pkl","permalink":"/blog/tags/pkl"}],"readingTime":8.27,"hasTruncateMarker":true,"authors":[{"name":"Miles Johnson","title":"Founder, developer","url":"https://github.com/milesj","imageURL":"/img/authors/miles.jpg","key":"milesj","page":null}],"frontMatter":{"slug":"moon-v1.29","title":"moon v1.29 - Improved affected tracking, experimental Pkl configuration, and more","authors":["milesj"],"tags":["affected","detection","tracker","project","task","config","pkl"],"image":"./img/moon/v1.29.png"},"unlisted":false,"prevItem":{"title":"proto v0.42 - New bin linking, JSON/YAML plugins, and more","permalink":"/blog/proto-v0.42"},"nextItem":{"title":"moon v1.28 - Task presets, OS tasks, meta tokens, and more","permalink":"/blog/moon-v1.28"}}')},97265:(e,n,t)=>{t.d(n,{A:()=>o});t(63696);var a=t(11750);const s={tabItem:"tabItem_wHwb"};var r=t(62540);function o(e){let{children:n,hidden:t,className:o}=e;return(0,r.jsx)("div",{role:"tabpanel",className:(0,a.A)(s.tabItem,o),hidden:t,children:n})}},97635:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>d,contentTitle:()=>c,default:()=>p,frontMatter:()=>l,metadata:()=>a,toc:()=>u});var a=t(92706),s=t(62540),r=t(43023),o=t(65457),i=t(97265);const l={slug:"moon-v1.29",title:"moon v1.29 - Improved affected tracking, experimental Pkl configuration, and more",authors:["milesj"],tags:["affected","detection","tracker","project","task","config","pkl"],image:"./img/moon/v1.29.png"},c=void 0,d={image:t(69386).A,authorsImageUrls:[void 0]},u=[{value:"New affected projects tracker",id:"new-affected-projects-tracker",level:2},{value:"Control upstream / downstream depth",id:"control-upstream--downstream-depth",level:3},{value:"What about tasks?",id:"what-about-tasks",level:3},{value:"Experimental support for Pkl based configuration",id:"experimental-support-for-pkl-based-configuration",level:2},{value:"Advanced examples",id:"advanced-examples",level:3},{value:"Caveats and restrictions",id:"caveats-and-restrictions",level:3},{value:"How to use Pkl?",id:"how-to-use-pkl",level:3},{value:"What about X instead?",id:"what-about-x-instead",level:3},{value:"Looking for contributors!",id:"looking-for-contributors",level:2},{value:"Other changes",id:"other-changes",level:2}];function h(e){const n={a:"a",blockquote:"blockquote",code:"code",em:"em",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,r.R)(),...e.components};return(0,s.jsxs)(s.Fragment,{children:[(0,s.jsx)(n.p,{children:"In this release, we're excited to introduce an improved affected tracker and a new (but\nexperimental) configuration format!"}),"\n",(0,s.jsx)(n.h2,{id:"new-affected-projects-tracker",children:"New affected projects tracker"}),"\n",(0,s.jsx)(n.p,{children:"We've received a lot of feedback that our affected projects and tasks logic works differently across\ncommands, or that it's hard to understand why something is affected or not affected. We wanted to\nadd more clarity around affected projects, so have implemented a new affected tracker."}),"\n",(0,s.jsx)(n.p,{children:'This new tracker includes a ton of new logging that we believe will answer the "why". For example,\nonce the tracker has finished tracking, we\'ll log all affected projects and tasks, and what marked\ntheir affected state.'}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-shell",children:'[DEBUG] moon_affected::affected_tracker  Project website is affected by  files=["website/blog/2024-10-01_moon-v1.29.mdx"] upstream=[] downstream=[] other=false\n[DEBUG] moon_affected::affected_tracker  Project runtime is affected by files=[] upstream=[] downstream=["website"] other=false\n[DEBUG] moon_affected::affected_tracker  Project types is affected by  files=[] upstream=[] downstream=["website", "runtime"] other=false\n[DEBUG] moon_affected::affected_tracker  Task runtime:build is affected by  env=[] files=[] upstream=[] downstream=["website:start", "website:build"] other=false\n[DEBUG] moon_affected::affected_tracker  Task website:start is affected by  env=[] files=["website/blog/2024-10-01_moon-v1.29.mdx"] upstream=[] downstream=[] other=false\n[DEBUG] moon_affected::affected_tracker  Task types:build is affected by  env=[] files=[] upstream=[] downstream=["website:start", "runtime:build", "website:build"] other=false\n[DEBUG] moon_affected::affected_tracker  Task website:build is affected by  env=[] files=["website/blog/2024-10-01_moon-v1.29.mdx"] upstream=[] downstream=[] other=false\n'})}),"\n",(0,s.jsx)(n.p,{children:"What marks an affected state is based on one or many of the following:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsx)(n.li,{children:"By touched files"}),"\n",(0,s.jsx)(n.li,{children:"By environment variables (task only)"}),"\n",(0,s.jsx)(n.li,{children:"By upstream dependencies"}),"\n",(0,s.jsx)(n.li,{children:"By downstream dependents (project only)"}),"\n",(0,s.jsx)(n.li,{children:"And other minor internal logic"}),"\n"]}),"\n",(0,s.jsxs)(n.p,{children:["This information is also included in the run report at ",(0,s.jsx)(n.code,{children:".moon/cache/runReport.json"}),", under the\n",(0,s.jsx)(n.code,{children:"context.affected"})," property. An example of this looks like:"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-json",children:'{\n  "projects": {\n    "website": {\n      "files": ["website/blog/2024-10-01_moon-v1.29.mdx"],\n      "other": true\n    },\n    "runtime": {\n      "downstream": ["website"],\n      "other": true\n    },\n    "types": {\n      "downstream": ["website", "runtime"],\n      "other": true\n    }\n  },\n  "tasks": {\n    "website:build": {\n      "files": ["website/blog/2024-10-01_moon-v1.29.mdx"],\n      "other": false\n    },\n    "types:build": {\n      "downstream": ["website:build"],\n      "other": false\n    },\n    "runtime:build": {\n      "downstream": ["website:build"],\n      "other": false\n    }\n  }\n}\n'})}),"\n",(0,s.jsx)(n.h3,{id:"control-upstream--downstream-depth",children:"Control upstream / downstream depth"}),"\n",(0,s.jsxs)(n.p,{children:["With this new tracker, we now have the ability to control the traversal depth for upstream\ndependencies and downstream dependents in ",(0,s.jsx)(n.code,{children:"moon query projects"}),", via the ",(0,s.jsx)(n.code,{children:"--upstream"})," and\n",(0,s.jsx)(n.code,{children:"--downstream"})," options respectively (the ",(0,s.jsx)(n.code,{children:"--dependents"})," option is now deprecated)."]}),"\n",(0,s.jsx)(n.p,{children:"These options support the following values:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"none"})," - Do not traverse deps."]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"direct"})," - Traverse direct parent/child deps."]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"deep"})," - Traverse full hierarchy deps."]}),"\n"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-shell",children:"$ moon query projects --affected --upstream none --downstream deep\n"})}),"\n",(0,s.jsx)(n.h3,{id:"what-about-tasks",children:"What about tasks?"}),"\n",(0,s.jsx)(n.p,{children:"We have the existing affected logic that has powered moon for years, and have updated that to\ninclude the new logging. However, it's not perfect and we want to improve it."}),"\n",(0,s.jsx)(n.p,{children:"To support this overall enhancement for tasks, we need to support a task graph, which we currently\ndo not. We only have a project graph (which has tasks), and an action graph (which has more than\ntasks). In a future release, we'll introduce a new task graph that will fill the gaps."}),"\n",(0,s.jsx)(n.h2,{id:"experimental-support-for-pkl-based-configuration",children:"Experimental support for Pkl based configuration"}),"\n",(0,s.jsxs)(n.p,{children:["Pkl, what is that? If you haven't heard of Pkl yet,\n",(0,s.jsx)(n.a,{href:"https://pkl-lang.org/",children:"Pkl is a programmable configuration format by Apple"}),". But what about YAML?\nYAML has served us well since the beginning, but we're not happy with YAML. It's better than JSON,\nTOML, and XML, but still has its downsides. We want something better, something that meets the\nfollowing requirements:"]}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsx)(n.li,{children:"Is easy to read and write."}),"\n",(0,s.jsx)(n.li,{children:"Is dynamic and programmable (loops, variables, etc)."}),"\n",(0,s.jsx)(n.li,{children:"Has type-safety or built-in schema support."}),"\n",(0,s.jsx)(n.li,{children:"Has Rust serde integration."}),"\n"]}),"\n",(0,s.jsxs)(n.p,{children:["The primary requirement that we are hoping to achieve is adopting a configuration format that is\n",(0,s.jsx)(n.em,{children:"programmable"}),". We want something that has native support for variables, loops, conditions, and\nmore, so that you could curate and compose your configuration very easily. Hacking this\nfunctionality into YAML is a terrible user experience in our opinion!"]}),"\n",(0,s.jsx)(n.p,{children:"And with all that said, I'm sure you're curious what Pkl actually looks like in practice. Here's a\nfew examples (unfortunately no syntax highlighting)!"}),"\n",(0,s.jsxs)(o.A,{defaultValue:"project",values:[{label:"moon.pkl",value:"project"},{label:".moon/workspace.pkl",value:"workspace"},{label:".moon/toolchain.pkl",value:"toolchain"}],children:[(0,s.jsx)(i.A,{value:"project",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-pkl",children:'type = "application"\nlanguage = "typescript"\ndependsOn = List("client", "ui")\n\ntasks {\n  ["build"] {\n    command = "docusaurus build"\n    deps = List("^:build")\n    outputs = List("build")\n    options {\n      interactive = true\n      retryCount = 3\n    }\n  }\n  ["typecheck"] {\n    command = "tsc --build"\n    inputs = new Listing {\n      "@globs(sources)"\n      "@globs(tests)"\n      "tsconfig.json"\n      "/tsconfig.options.json"\n    }\n  }\n}\n'})})}),(0,s.jsx)(i.A,{value:"workspace",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-pkl",children:'projects {\n  globs = List("apps/*", "packages/*")\n  sources {\n    ["root"] = "."\n  }\n}\n\nvcs {\n  defaultBranch = "master"\n}\n'})})}),(0,s.jsx)(i.A,{value:"toolchain",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-pkl",children:'node {\n  version = "20.15.0"\n  packageManager = "yarn"\n  yarn {\n    version = "4.3.1"\n  }\n  addEnginesConstraint = false\n  inferTasksFromScripts = false\n}\n'})})})]}),"\n",(0,s.jsx)(n.p,{children:"Pretty straight forward for the most part! Lists/Listings (arrays) are a bit different than what you\nmay be used to, but they're super easy to learn."}),"\n",(0,s.jsx)(n.h3,{id:"advanced-examples",children:"Advanced examples"}),"\n",(0,s.jsx)(n.p,{children:"I've talked a lot about programmable configs, but what exactly does that look like? Let's go through\na few examples. Say you are building a Rust crate and you need a build task for each operating\nsystem. In YAML you would need to define each of these manually, but with Pkl, you can build it with\na loop!"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-pkl",children:'tasks {\n  for (_os in List("linux", "macos", "windows")) {\n    ["build-\\(_os)"] {\n      command = "cargo"\n      args = List(\n        "--target",\n        if (_os == "linux") "x86_64-unknown-linux-gnu"\n          else if (_os == "macos") "x86_64-apple-darwin"\n          else "i686-pc-windows-msvc",\n        "--verbose"\n      )\n      options {\n        os = _os\n      }\n    }\n  }\n}\n'})}),"\n",(0,s.jsxs)(n.p,{children:["Or maybe you want to share inputs across multiple tasks. This can be achieved with ",(0,s.jsx)(n.code,{children:"local"}),"\nvariables."]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-pkl",children:'local _sharedInputs = List("src/**/*")\n\ntasks {\n  ["test"] {\n    // ...\n    inputs = List("tests/**/*") + _sharedInputs\n  }\n  ["lint"] {\n    // ...\n    inputs = List("**/*.graphql") + _sharedInputs\n  }\n}\n'})}),"\n",(0,s.jsxs)(n.p,{children:["Pretty awesome right? This is just a taste of what Pkl has to offer! We highly suggest reading the\n",(0,s.jsx)(n.a,{href:"https://pkl-lang.org/main/current/language-reference/index.html",children:"language reference"}),", the\n",(0,s.jsx)(n.a,{href:"https://pkl-lang.org/main/current/standard-library.html",children:"standard library"}),", or looking at our\n",(0,s.jsx)(n.a,{href:"https://github.com/moonrepo/moon/tree/master/crates/config/tests/__fixtures__/pkl",children:"example configurations"}),"\nwhile testing Pkl."]}),"\n",(0,s.jsxs)(n.blockquote,{children:["\n",(0,s.jsx)(n.p,{children:"In the future, if Pkl seems like the right fit, we plan to take full advantage of what it has to\noffer, by creating our own Pkl projects, modules, and types!"}),"\n"]}),"\n",(0,s.jsx)(n.h3,{id:"caveats-and-restrictions",children:"Caveats and restrictions"}),"\n",(0,s.jsx)(n.p,{children:"Since this is an entirely new configuration format that is quite dynamic compared to YAML, there are\nsome key differences to be aware of!"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["\n",(0,s.jsxs)(n.p,{children:["Each ",(0,s.jsx)(n.code,{children:".pkl"})," file is evaluated in isolation (loops are processed, variables assigned, etc). This\nmeans that task inheritance and file merging cannot extend or infer this native functionality."]}),"\n"]}),"\n",(0,s.jsxs)(n.li,{children:["\n",(0,s.jsxs)(n.p,{children:[(0,s.jsx)(n.code,{children:"default"})," is a\n",(0,s.jsx)(n.a,{href:"https://pkl-lang.org/main/current/language-reference/index.html#default-element",children:"special feature"}),"\nin Pkl and cannot be used as a setting name. This only applies to\n",(0,s.jsx)(n.a,{href:"/docs/config/template#default",children:(0,s.jsx)(n.code,{children:"template.yml"})}),", but can be worked around by using ",(0,s.jsx)(n.code,{children:"defaultValue"}),"\ninstead."]}),"\n"]}),"\n"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-pkl",metastring:'title="template.yml"',children:'variables {\n  ["age"] {\n    type = "number"\n    prompt = "Age?"\n    defaultValue = 0\n}\n'})}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.code,{children:"local"})," is also a reserved word in Pkl. It can be worked around by escaping it with backticks, or\nyou can simply use the ",(0,s.jsxs)(n.a,{href:"/docs/config/project#preset",children:[(0,s.jsx)(n.code,{children:"preset"})," setting"]})," instead."]}),"\n"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-pkl",children:'tasks {\n  ["example"] {\n    `local` = true\n    # Or\n    preset = "server"\n  }\n}\n'})}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsx)(n.li,{children:"Only files are supported. Cannot use or extend from URLs."}),"\n"]}),"\n",(0,s.jsx)(n.h3,{id:"how-to-use-pkl",children:"How to use Pkl?"}),"\n",(0,s.jsxs)(n.p,{children:["As mentioned in the heading, Pkl support is experimental, and ",(0,s.jsx)(n.em,{children:"is not"})," enabled by default. If you're\ninterested in trying out Pkl, you can with the following:"]}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsxs)(n.a,{href:"https://pkl-lang.org/main/current/pkl-cli/index.html#installation",children:["Install ",(0,s.jsx)(n.code,{children:"pkl"})," onto ",(0,s.jsx)(n.code,{children:"PATH"})]}),".\nPkl uses a client-server communication model.","\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["Can also be installed with proto:\n",(0,s.jsx)(n.code,{children:"proto plugin add pkl https://raw.githubusercontent.com/milesj/proto-plugins/refs/heads/master/pkl.toml"})]}),"\n"]}),"\n"]}),"\n",(0,s.jsxs)(n.li,{children:["Use the ",(0,s.jsx)(n.code,{children:".pkl"})," file extension instead of ",(0,s.jsx)(n.code,{children:".yml"}),"."]}),"\n",(0,s.jsxs)(n.li,{children:["Pass the ",(0,s.jsx)(n.code,{children:"--experimentPklConfig"})," CLI option, or set the ",(0,s.jsx)(n.code,{children:"MOON_EXPERIMENT_PKL_CONFIG"})," environment\nvariable."]}),"\n"]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-shell",children:"$ moon check --all --experimentPklConfig\n# Or\n$ MOON_EXPERIMENT_PKL_CONFIG=true moon check --all\n"})}),"\n",(0,s.jsxs)(n.blockquote,{children:["\n",(0,s.jsx)(n.p,{children:"Pkl can be used alongside YAML with no issues! We'll merge, inherit, and compose as usual."}),"\n"]}),"\n",(0,s.jsx)(n.h3,{id:"what-about-x-instead",children:"What about X instead?"}),"\n",(0,s.jsx)(n.p,{children:"There are a handful of other interesting or popular programmable configurations out there, so why\nisn't moon experimenting with those? The answer is, we may! Just so long as they meet the\nrequirements. With that said, we do have some opinions below:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.a,{href:"https://github.com/bazelbuild/starlark/",children:"Starlark/Skylark"})," - On our list to evaluate."]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.a,{href:"https://nickel-lang.org",children:"Nickel"}),", ",(0,s.jsx)(n.a,{href:"https://jsonnet.org",children:"Jsonnet"})," - On our list to evaluate, but\nnot a fan of the JSON-like syntax."]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.a,{href:"https://dhall-lang.org",children:"Dhall"})," - While this meets most of our requirements, the syntax isn't as\nreadable or user-friendly as we'd like."]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.a,{href:"https://cuelang.org/",children:"CUE"})," - No Rust support, so unlikely. It also works quite differently than\nthe other tools."]}),"\n",(0,s.jsxs)(n.li,{children:[(0,s.jsx)(n.a,{href:"https://www.kcl-lang.io/",children:"KCL"})," - Nice syntax and meets the requirements, but no Rust support."]}),"\n"]}),"\n",(0,s.jsx)(n.p,{children:"If there's another format you think we should investigate, drop us a line in Discord!"}),"\n",(0,s.jsx)(n.h2,{id:"looking-for-contributors",children:"Looking for contributors!"}),"\n",(0,s.jsx)(n.p,{children:"Are you a fan of moon (or proto)? Interested in learning Rust or writing more Rust? Want to\ncontribute to an awesome project (we think so)? Well it just so happens that we are looking for\nactive contributors!"}),"\n",(0,s.jsx)(n.p,{children:"We have a very long roadmap of features we would like to implement, but do not have enough time or\nresources to implement them in the timeframe we would like. These features range from very small\n(low hanging fruit) to very large (and quite complex)."}),"\n",(0,s.jsx)(n.p,{children:"If this sounds like something you may be interested in, post a message in Discord and let us know!\nOnly a few hours a week commitment is good enough for us."}),"\n",(0,s.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,s.jsxs)(n.p,{children:["View the ",(0,s.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/v1.29.0",children:"official release"})," for a full list\nof changes."]}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["Added a new task option, ",(0,s.jsx)(n.code,{children:"cacheLifetime"}),", that controls how long a task will be cached for."]}),"\n",(0,s.jsxs)(n.li,{children:["Added a new task merge strategy, ",(0,s.jsx)(n.code,{children:"preserve"}),", that preserves the original inherited value."]}),"\n",(0,s.jsxs)(n.li,{children:["Added a new setting ",(0,s.jsx)(n.code,{children:"vcs.hookFormat"})," to ",(0,s.jsx)(n.code,{children:".moon/workspace.yml"}),", that can customize the shell/file\nformat for hooks."]}),"\n",(0,s.jsxs)(n.li,{children:["Updated task ",(0,s.jsx)(n.code,{children:"outputs"})," to support token and environment variables."]}),"\n",(0,s.jsxs)(n.li,{children:["Updated ",(0,s.jsx)(n.code,{children:"moon query projects"})," to include the project description as a trailing value."]}),"\n",(0,s.jsxs)(n.li,{children:["Updated ",(0,s.jsx)(n.code,{children:"moon query tasks"})," to include the task type and platform, and the task description as a\ntrailing value."]}),"\n"]})]})}function p(e={}){const{wrapper:n}={...(0,r.R)(),...e.components};return n?(0,s.jsx)(n,{...e,children:(0,s.jsx)(h,{...e})}):h(e)}}}]);