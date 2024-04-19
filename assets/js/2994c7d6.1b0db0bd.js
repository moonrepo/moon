"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[83088],{27721:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>u,contentTitle:()=>c,default:()=>p,frontMatter:()=>l,metadata:()=>d,toc:()=>h});var s=t(24246),a=t(71670),o=t(33337),r=t(39798),i=t(76911);const l={slug:"v0.23",title:"moon v0.23 - Scoped task inheritance, and project config updates",authors:["milesj"],tags:["survey","tasks","projects"],image:"./img/v0.23.png"},c=void 0,d={permalink:"/blog/v0.23",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2023-01-30_v0.23.mdx",source:"@site/blog/2023-01-30_v0.23.mdx",title:"moon v0.23 - Scoped task inheritance, and project config updates",description:"With this release, we're launching the next iteration of our task inheritance model, as well as",date:"2023-01-30T00:00:00.000Z",tags:[{label:"survey",permalink:"/blog/tags/survey"},{label:"tasks",permalink:"/blog/tags/tasks"},{label:"projects",permalink:"/blog/tags/projects"}],readingTime:5.74,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"v0.23",title:"moon v0.23 - Scoped task inheritance, and project config updates",authors:["milesj"],tags:["survey","tasks","projects"],image:"./img/v0.23.png"},unlisted:!1,prevItem:{title:"Remote caching is now publicly available through moonbase",permalink:"/blog/moonbase"},nextItem:{title:"moon v0.22 - New pipeline, hashing, and caching, with Turborepo migration",permalink:"/blog/v0.22"}},u={image:t(57861).Z,authorsImageUrls:[void 0]},h=[{value:"Developer survey",id:"developer-survey",level:2},{value:"Improved task inheritance model",id:"improved-task-inheritance-model",level:2},{value:"New <code>.moon/tasks.yml</code> (breaking)",id:"new-moontasksyml-breaking",level:3},{value:"New scoped tasks with <code>.moon/tasks/*.yml</code>",id:"new-scoped-tasks-with-moontasksyml",level:3},{value:"Moved <code>implicitDeps</code> and <code>implicitInputs</code> (breaking)",id:"moved-implicitdeps-and-implicitinputs-breaking",level:3},{value:"Project-level environment variables",id:"project-level-environment-variables",level:2},{value:"Globs in task outputs",id:"globs-in-task-outputs",level:2},{value:"Other changes",id:"other-changes",level:2},{value:"What&#39;s next?",id:"whats-next",level:2}];function m(e){const n={a:"a",blockquote:"blockquote",code:"code",em:"em",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",ul:"ul",...(0,a.a)(),...e.components};return(0,s.jsxs)(s.Fragment,{children:[(0,s.jsx)(n.p,{children:"With this release, we're launching the next iteration of our task inheritance model, as well as\nquality of life improvements for project configuration."}),"\n",(0,s.jsx)(n.h2,{id:"developer-survey",children:"Developer survey"}),"\n",(0,s.jsx)(n.p,{children:"Before we dive into this new release, we have a quick survey for everyone. We know how everyone\nfeels about surveys, but this one is real quick, only a few minutes, and is mostly multiple choice\nquestions."}),"\n",(0,s.jsx)(n.p,{children:"We're looking for feedback on moon itself, what features you're looking for, what you currently do\nnot like, how you're currently using monorepos, your development workflows, so on and so forth. We'd\nvery much appreciate it if you could engage with this survey!"}),"\n",(0,s.jsx)("div",{class:"flex justify-center",children:(0,s.jsx)(i.Z,{label:"Take survey!",href:"https://a.sprig.com/UE1SOG1zV3o5SzdRfnNpZDpmOTQ5MjU1Yy1jYTZlLTRmYjQtOTRjZi0wMzZlZjExN2JjZDg=",size:"lg"})}),"\n",(0,s.jsx)(n.h2,{id:"improved-task-inheritance-model",children:"Improved task inheritance model"}),"\n",(0,s.jsxs)(n.p,{children:['One of the guiding principles behind moon is to simplify repository maintenance, with task\nmanagement being top of list. We weren\'t happy with the current state of things, as every build\nsystem and task runner that exists always opted for per-project task management, which is a massive\namount of overhead and tech debt in the long run. To combat this, moon was designed from the\nground-up using a task inheritance model, where "global" tasks were defined in\n',(0,s.jsx)(n.a,{href:"/docs/config/tasks",children:(0,s.jsx)(n.code,{children:".moon/project.yml"})}),", with per-project tasks still being an option with\n",(0,s.jsx)(n.a,{href:"/docs/config/project",children:(0,s.jsx)(n.code,{children:"moon.yml"})}),"."]}),"\n",(0,s.jsx)(n.p,{children:"While inheritance worked great, it did have some shortcomings, such as:"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["With the addition of ",(0,s.jsx)(n.a,{href:"/blog/v0.21",children:"new programming languages"}),", there's no way to easily define\ntasks for specific languages, that should only be inherited by specific projects."]}),"\n",(0,s.jsx)(n.li,{children:"There's no way to differentiate tasks between applications or libraries, as they typically have\ndifferent build/compilation systems."}),"\n",(0,s.jsxs)(n.li,{children:['All of the problems above can be "solved" with\n',(0,s.jsx)(n.a,{href:"/docs/config/project#inheritedtasks",children:(0,s.jsx)(n.code,{children:"workspace.inheritedTasks"})})," in all projects, but it's a\nmaintenance headache."]}),"\n"]}),"\n",(0,s.jsx)(n.p,{children:"We've been documenting a solution to these problems for many months now, and we're very excited to\nfinally release our new and improved task inheritance model that solves all of the problems above,\nand opens the doors for future enhancements! Keep reading for more information."}),"\n",(0,s.jsxs)(n.h3,{id:"new-moontasksyml-breaking",children:["New ",(0,s.jsx)(n.code,{children:".moon/tasks.yml"})," (breaking)"]}),"\n",(0,s.jsxs)(n.p,{children:["To start, we renamed ",(0,s.jsx)(n.code,{children:".moon/project.yml"})," to ",(0,s.jsx)(n.code,{children:".moon/tasks.yml"})," as we want to emphasize that this\nconfiguration file is for task inheritance functionality only. However, the semantics of this file\nhas ",(0,s.jsx)(n.em,{children:"not"}),' changed, and is still "tasks to be inherited by ',(0,s.jsx)(n.em,{children:"all"}),' projects".']}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks.yml"',children:"$schema: 'https://moonrepo.dev/schemas/tasks.json'\n\ntasks:\n  # ...\n"})}),"\n",(0,s.jsxs)(n.blockquote,{children:["\n",(0,s.jsxs)(n.p,{children:["We'll automatically rename this file for you when running a ",(0,s.jsx)(n.code,{children:"moon"})," command!"]}),"\n"]}),"\n",(0,s.jsxs)(n.h3,{id:"new-scoped-tasks-with-moontasksyml",children:["New scoped tasks with ",(0,s.jsx)(n.code,{children:".moon/tasks/*.yml"})]}),"\n",(0,s.jsxs)(n.p,{children:["The biggest change to task inheritance is that tasks can now be scoped by a project's\n",(0,s.jsx)(n.a,{href:"/docs/config/project#language",children:(0,s.jsx)(n.code,{children:"language"})})," or ",(0,s.jsx)(n.a,{href:"/docs/config/project#type",children:(0,s.jsx)(n.code,{children:"type"})})," using the new\n",(0,s.jsx)(n.code,{children:".moon/tasks/<language>.yml"})," or ",(0,s.jsx)(n.code,{children:".moon/tasks/<language>-<type>.yml"})," configuration files! Jump to the\n",(0,s.jsx)(n.a,{href:"/docs/concepts/task-inheritance",children:"official documentation on task inheritance"})," for more information\non how scoping works, the lookup order of files, and much more."]}),"\n",(0,s.jsxs)(n.p,{children:["As a demonstration, you can scope tasks to Node.js projects with ",(0,s.jsx)(n.code,{children:".moon/tasks/node.yml"}),", Rust\napplications with ",(0,s.jsx)(n.code,{children:".moon/tasks/rust-application.yml"}),", Go libraries with\n",(0,s.jsx)(n.code,{children:".moon/tasks/go-library.yml"}),", Ruby scripts with ",(0,s.jsx)(n.code,{children:".moon/tasks/ruby-tool.yml"}),", so on and so forth!"]}),"\n",(0,s.jsx)(n.p,{children:"We're very excited for this feature, as it's something we personally needed, and we're sure you all\ndo as well. It also future proofs moon for new programming languages, additional implicit scenarios\nto handle, and yet to be discovered functionality."}),"\n",(0,s.jsxs)(o.Z,{groupId:"scoped-task",defaultValue:"node",values:[{label:"Node",value:"node"},{label:"Go",value:"go"},{label:"PHP",value:"php"},{label:"Python",value:"python"},{label:"Ruby",value:"ruby"},{label:"Rust",value:"rust"}],children:[(0,s.jsx)(r.Z,{value:"node",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/node.yml"',children:"tasks:\n  format:\n    command: 'prettier --write .'\n"})})}),(0,s.jsx)(r.Z,{value:"go",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/go.yml"',children:"tasks:\n  format:\n    command: 'go fmt'\n"})})}),(0,s.jsx)(r.Z,{value:"php",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/php.yml"',children:"tasks:\n  format:\n    command: 'phpcbf .'\n"})})}),(0,s.jsx)(r.Z,{value:"python",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/python.yml"',children:"tasks:\n  format:\n    command: 'pylint .'\n"})})}),(0,s.jsx)(r.Z,{value:"ruby",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/ruby.yml"',children:"tasks:\n  format:\n    command: 'rubocop -l'\n"})})}),(0,s.jsx)(r.Z,{value:"rust",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/rust.yml"',children:"tasks:\n  format:\n    command: 'cargo fmt --all --check'\n"})})})]}),"\n",(0,s.jsxs)(n.h3,{id:"moved-implicitdeps-and-implicitinputs-breaking",children:["Moved ",(0,s.jsx)(n.code,{children:"implicitDeps"})," and ",(0,s.jsx)(n.code,{children:"implicitInputs"})," (breaking)"]}),"\n",(0,s.jsxs)(n.p,{children:["To standardize inheritance and expansion related functionality, we've moved the\n",(0,s.jsx)(n.code,{children:"runner.implicitDeps"})," and ",(0,s.jsx)(n.code,{children:"runner.implicitInputs"})," settings from ",(0,s.jsx)(n.code,{children:".moon/workspace.yml"})," to\n",(0,s.jsx)(n.a,{href:"/docs/config/tasks#implicitdeps",children:(0,s.jsx)(n.code,{children:".moon/tasks.yml"})})," and\n",(0,s.jsx)(n.a,{href:"/docs/config/tasks#implicitinputs",children:(0,s.jsx)(n.code,{children:".moon/tasks/*.yml"})})," and removed the ",(0,s.jsx)(n.code,{children:"runner"})," prefix."]}),"\n",(0,s.jsx)(n.p,{children:"This allows for implicits to also be scoped accordingly and granularly. For example, projects can\nnow inherit dependency manager related files as implicit inputs on a per-language basis:"}),"\n",(0,s.jsxs)(o.Z,{groupId:"scoped-task",defaultValue:"node",values:[{label:"Node",value:"node"},{label:"Go",value:"go"},{label:"PHP",value:"php"},{label:"Python",value:"python"},{label:"Ruby",value:"ruby"},{label:"Rust",value:"rust"}],children:[(0,s.jsx)(r.Z,{value:"node",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/node.yml"',children:"implicitInputs:\n  - 'package.json'\n"})})}),(0,s.jsx)(r.Z,{value:"go",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/go.yml"',children:"implicitInputs:\n  - 'go.mod'\n"})})}),(0,s.jsx)(r.Z,{value:"php",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/php.yml"',children:"implicitInputs:\n  - 'composer.json'\n"})})}),(0,s.jsx)(r.Z,{value:"python",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/python.yml"',children:"implicitInputs:\n  - 'pyproject.toml'\n"})})}),(0,s.jsx)(r.Z,{value:"ruby",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/ruby.yml"',children:"implicitInputs:\n  - 'Gemfile'\n"})})}),(0,s.jsx)(r.Z,{value:"rust",children:(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title=".moon/tasks/rust.yml"',children:"implicitInputs:\n  - 'Cargo.toml'\n"})})})]}),"\n",(0,s.jsx)(n.h2,{id:"project-level-environment-variables",children:"Project-level environment variables"}),"\n",(0,s.jsxs)(n.p,{children:["Since moon's inception, tasks can be configured with pre-defined environment variables using the\n",(0,s.jsx)(n.a,{href:"/docs/config/project#env-1",children:(0,s.jsx)(n.code,{children:"env"})})," setting. These variables would then be passed to the command\nduring execution. This works perfectly for encapsulation, but becomes tedious when the same\nvariables are repeated for multiple tasks."]}),"\n",(0,s.jsxs)(n.p,{children:["To remedy this, environment variables can now be defined at the top of\n",(0,s.jsx)(n.a,{href:"/docs/config/project",children:(0,s.jsx)(n.code,{children:"moon.yml"})})," using the top-level ",(0,s.jsx)(n.a,{href:"/docs/config/project#env",children:(0,s.jsx)(n.code,{children:"env"})})," setting.\nVariables defined at the top-level will be inherited by all tasks in the current project, but will\nnot override task-level variables of the same name."]}),"\n",(0,s.jsx)(n.p,{children:"To demonstrate this, the following config:"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title="<project>/moon.yml"',children:"tasks:\n  dev:\n    # ...\n    env:\n      TARGET_ENV: 'development'\n\n  build:\n    # ...\n    env:\n      TARGET_ENV: 'development'\n\n  serve:\n    # ...\n    env:\n      TARGET_ENV: 'development'\n"})}),"\n",(0,s.jsx)(n.p,{children:"Can be rewritten as:"}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title="<project>/moon.yml"',children:"env:\n  TARGET_ENV: 'development'\n\ntasks:\n  dev:\n    # ...\n\n  build:\n    # ...\n\n  serve:\n    # ...\n"})}),"\n",(0,s.jsx)(n.h2,{id:"globs-in-task-outputs",children:"Globs in task outputs"}),"\n",(0,s.jsxs)(n.p,{children:["Another feature that's been around since moon's inception is task\n",(0,s.jsx)(n.a,{href:"/docs/config/project#outputs",children:(0,s.jsx)(n.code,{children:"outputs"})}),", which only supported relative files and folders. For\nhistorical reasons, it was the easiest solution at the time, but in practice, supporting more\ngranular control is better."]}),"\n",(0,s.jsxs)(n.p,{children:["As such, task ",(0,s.jsx)(n.code,{children:"outputs"})," now support glob patterns as well! This is perfect for restricting and\nfiltering down which files are cached in the artifact. However, be aware that during hydration (a\ncache hit), all files ",(0,s.jsx)(n.em,{children:"not matching the glob"})," will be deleted, so ensure that critical files ",(0,s.jsx)(n.em,{children:"do"}),"\nmatch."]}),"\n",(0,s.jsxs)(n.p,{children:["To demonstrate this, if building a JavaScript project, you may want to include ",(0,s.jsx)(n.code,{children:".js"})," and ",(0,s.jsx)(n.code,{children:".css"}),"\nfiles, but exclude everything else (",(0,s.jsx)(n.code,{children:".map"}),", etc)."]}),"\n",(0,s.jsx)(n.pre,{children:(0,s.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml" {4,5}',children:"tasks:\n  build:\n    command: 'webpack'\n    outputs:\n      - 'build/**/*.{js,css}'\n"})}),"\n",(0,s.jsx)(n.h2,{id:"other-changes",children:"Other changes"}),"\n",(0,s.jsxs)(n.p,{children:["View the\n",(0,s.jsx)(n.a,{href:"https://github.com/moonrepo/moon/releases/tag/%40moonrepo%2Fcli%400.23.0",children:"official release"})," for a\nfull list of changes."]}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["Updated ",(0,s.jsx)(n.code,{children:"moon migrate from-turborepo"})," to preserve globs in outputs."]}),"\n",(0,s.jsx)(n.li,{children:"Updated project graph to no longer cache when there's no VCS root."}),"\n",(0,s.jsxs)(n.li,{children:["Updated pnpm to use the new ",(0,s.jsx)(n.code,{children:"pnpm dedupe"})," command when the version is >= 7.26.0."]}),"\n"]}),"\n",(0,s.jsx)(n.h2,{id:"whats-next",children:"What's next?"}),"\n",(0,s.jsx)(n.p,{children:"Expect the following in the v0.24 release!"}),"\n",(0,s.jsxs)(n.ul,{children:["\n",(0,s.jsxs)(n.li,{children:["New ",(0,s.jsx)(n.code,{children:"moon query tasks"})," command."]}),"\n",(0,s.jsxs)(n.li,{children:["New per-project ",(0,s.jsx)(n.code,{children:"platform"})," setting."]}),"\n",(0,s.jsxs)(n.li,{children:["Token support in task ",(0,s.jsx)(n.code,{children:"outputs"}),"."]}),"\n",(0,s.jsx)(n.li,{children:"TypeScript v5 support."}),"\n"]})]})}function p(e={}){const{wrapper:n}={...(0,a.a)(),...e.components};return n?(0,s.jsx)(n,{...e,children:(0,s.jsx)(m,{...e})}):m(e)}},39798:(e,n,t)=>{t.d(n,{Z:()=>r});t(27378);var s=t(40624);const a={tabItem:"tabItem_wHwb"};var o=t(24246);function r(e){let{children:n,hidden:t,className:r}=e;return(0,o.jsx)("div",{role:"tabpanel",className:(0,s.Z)(a.tabItem,r),hidden:t,children:n})}},33337:(e,n,t)=>{t.d(n,{Z:()=>m});var s=t(27378),a=t(40624),o=t(83457),r=t(35595),i=t(76457);const l={tabList:"tabList_J5MA",tabItem:"tabItem_l0OV"};var c=t(24246);function d(e){let{className:n,block:t,selectedValue:s,selectValue:r,tabValues:i}=e;const d=[],{blockElementScrollPositionUntilNextRender:u}=(0,o.o5)(),h=e=>{const n=e.currentTarget,t=d.indexOf(n),a=i[t].value;a!==s&&(u(n),r(a))},m=e=>{let n=null;switch(e.key){case"Enter":h(e);break;case"ArrowRight":{const t=d.indexOf(e.currentTarget)+1;n=d[t]??d[0];break}case"ArrowLeft":{const t=d.indexOf(e.currentTarget)-1;n=d[t]??d[d.length-1];break}}n?.focus()};return(0,c.jsx)("ul",{role:"tablist","aria-orientation":"horizontal",className:(0,a.Z)("tabs",{"tabs--block":t},n),children:i.map((e=>{let{value:n,label:t,attributes:o}=e;return(0,c.jsx)("li",{role:"tab",tabIndex:s===n?0:-1,"aria-selected":s===n,ref:e=>d.push(e),onKeyDown:m,onClick:h,...o,className:(0,a.Z)("tabs__item",l.tabItem,o?.className,{"tabs__item--active":s===n}),children:t??n},n)}))})}function u(e){let{lazy:n,children:t,selectedValue:a}=e;const o=(Array.isArray(t)?t:[t]).filter(Boolean);if(n){const e=o.find((e=>e.props.value===a));return e?(0,s.cloneElement)(e,{className:"margin-top--md"}):null}return(0,c.jsx)("div",{className:"margin-top--md",children:o.map(((e,n)=>(0,s.cloneElement)(e,{key:n,hidden:e.props.value!==a})))})}function h(e){const n=(0,r.Y)(e);return(0,c.jsxs)("div",{className:(0,a.Z)("tabs-container",l.tabList),children:[(0,c.jsx)(d,{...e,...n}),(0,c.jsx)(u,{...e,...n})]})}function m(e){const n=(0,i.Z)();return(0,c.jsx)(h,{...e,children:(0,r.h)(e.children)},String(n))}},35595:(e,n,t)=>{t.d(n,{Y:()=>m,h:()=>c});var s=t(27378),a=t(3620),o=t(9834),r=t(30654),i=t(70784),l=t(71819);function c(e){return s.Children.toArray(e).filter((e=>"\n"!==e)).map((e=>{if(!e||(0,s.isValidElement)(e)&&function(e){const{props:n}=e;return!!n&&"object"==typeof n&&"value"in n}(e))return e;throw new Error(`Docusaurus error: Bad <Tabs> child <${"string"==typeof e.type?e.type:e.type.name}>: all children of the <Tabs> component should be <TabItem>, and every <TabItem> should have a unique "value" prop.`)}))?.filter(Boolean)??[]}function d(e){const{values:n,children:t}=e;return(0,s.useMemo)((()=>{const e=n??function(e){return c(e).map((e=>{let{props:{value:n,label:t,attributes:s,default:a}}=e;return{value:n,label:t,attributes:s,default:a}}))}(t);return function(e){const n=(0,i.l)(e,((e,n)=>e.value===n.value));if(n.length>0)throw new Error(`Docusaurus error: Duplicate values "${n.map((e=>e.value)).join(", ")}" found in <Tabs>. Every value needs to be unique.`)}(e),e}),[n,t])}function u(e){let{value:n,tabValues:t}=e;return t.some((e=>e.value===n))}function h(e){let{queryString:n=!1,groupId:t}=e;const o=(0,a.k6)(),i=function(e){let{queryString:n=!1,groupId:t}=e;if("string"==typeof n)return n;if(!1===n)return null;if(!0===n&&!t)throw new Error('Docusaurus error: The <Tabs> component groupId prop is required if queryString=true, because this value is used as the search param name. You can also provide an explicit value such as queryString="my-search-param".');return t??null}({queryString:n,groupId:t});return[(0,r._X)(i),(0,s.useCallback)((e=>{if(!i)return;const n=new URLSearchParams(o.location.search);n.set(i,e),o.replace({...o.location,search:n.toString()})}),[i,o])]}function m(e){const{defaultValue:n,queryString:t=!1,groupId:a}=e,r=d(e),[i,c]=(0,s.useState)((()=>function(e){let{defaultValue:n,tabValues:t}=e;if(0===t.length)throw new Error("Docusaurus error: the <Tabs> component requires at least one <TabItem> children component");if(n){if(!u({value:n,tabValues:t}))throw new Error(`Docusaurus error: The <Tabs> has a defaultValue "${n}" but none of its children has the corresponding value. Available values are: ${t.map((e=>e.value)).join(", ")}. If you intend to show no default tab, use defaultValue={null} instead.`);return n}const s=t.find((e=>e.default))??t[0];if(!s)throw new Error("Unexpected error: 0 tabValues");return s.value}({defaultValue:n,tabValues:r}))),[m,p]=h({queryString:t,groupId:a}),[g,f]=function(e){let{groupId:n}=e;const t=function(e){return e?`docusaurus.tab.${e}`:null}(n),[a,o]=(0,l.Nk)(t);return[a,(0,s.useCallback)((e=>{t&&o.set(e)}),[t,o])]}({groupId:a}),j=(()=>{const e=m??g;return u({value:e,tabValues:r})?e:null})();(0,o.Z)((()=>{j&&c(j)}),[j]);return{selectedValue:i,selectValue:(0,s.useCallback)((e=>{if(!u({value:e,tabValues:r}))throw new Error(`Can't select invalid tab value=${e}`);c(e),p(e),f(e)}),[p,f,r]),tabValues:r}}},57861:(e,n,t)=>{t.d(n,{Z:()=>s});const s=t.p+"assets/images/v0.23-7f465b99a3ddadd6415b79205a586713.png"},71670:(e,n,t)=>{t.d(n,{Z:()=>i,a:()=>r});var s=t(27378);const a={},o=s.createContext(a);function r(e){const n=s.useContext(o);return s.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function i(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(a):e.components||a:r(e.components),s.createElement(o.Provider,{value:n},e.children)}}}]);