"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[2656],{35318:(e,t,n)=>{n.d(t,{Zo:()=>p,kt:()=>d});var r=n(27378);function o(e,t,n){return t in e?Object.defineProperty(e,t,{value:n,enumerable:!0,configurable:!0,writable:!0}):e[t]=n,e}function a(e,t){var n=Object.keys(e);if(Object.getOwnPropertySymbols){var r=Object.getOwnPropertySymbols(e);t&&(r=r.filter((function(t){return Object.getOwnPropertyDescriptor(e,t).enumerable}))),n.push.apply(n,r)}return n}function i(e){for(var t=1;t<arguments.length;t++){var n=null!=arguments[t]?arguments[t]:{};t%2?a(Object(n),!0).forEach((function(t){o(e,t,n[t])})):Object.getOwnPropertyDescriptors?Object.defineProperties(e,Object.getOwnPropertyDescriptors(n)):a(Object(n)).forEach((function(t){Object.defineProperty(e,t,Object.getOwnPropertyDescriptor(n,t))}))}return e}function s(e,t){if(null==e)return{};var n,r,o=function(e,t){if(null==e)return{};var n,r,o={},a=Object.keys(e);for(r=0;r<a.length;r++)n=a[r],t.indexOf(n)>=0||(o[n]=e[n]);return o}(e,t);if(Object.getOwnPropertySymbols){var a=Object.getOwnPropertySymbols(e);for(r=0;r<a.length;r++)n=a[r],t.indexOf(n)>=0||Object.prototype.propertyIsEnumerable.call(e,n)&&(o[n]=e[n])}return o}var l=r.createContext({}),c=function(e){var t=r.useContext(l),n=t;return e&&(n="function"==typeof e?e(t):i(i({},t),e)),n},p=function(e){var t=c(e.components);return r.createElement(l.Provider,{value:t},e.children)},u={inlineCode:"code",wrapper:function(e){var t=e.children;return r.createElement(r.Fragment,{},t)}},m=r.forwardRef((function(e,t){var n=e.components,o=e.mdxType,a=e.originalType,l=e.parentName,p=s(e,["components","mdxType","originalType","parentName"]),m=c(n),d=o,f=m["".concat(l,".").concat(d)]||m[d]||u[d]||a;return n?r.createElement(f,i(i({ref:t},p),{},{components:n})):r.createElement(f,i({ref:t},p))}));function d(e,t){var n=arguments,o=t&&t.mdxType;if("string"==typeof e||o){var a=n.length,i=new Array(a);i[0]=m;var s={};for(var l in t)hasOwnProperty.call(t,l)&&(s[l]=t[l]);s.originalType=e,s.mdxType="string"==typeof e?e:o,i[1]=s;for(var c=2;c<a;c++)i[c]=n[c];return r.createElement.apply(null,i)}return r.createElement.apply(null,n)}m.displayName="MDXCreateElement"},29759:(e,t,n)=>{n.r(t),n.d(t,{assets:()=>l,contentTitle:()=>i,default:()=>u,frontMatter:()=>a,metadata:()=>s,toc:()=>c});var r=n(25773),o=(n(27378),n(35318));const a={title:"Root-level project"},i=void 0,s={unversionedId:"guides/root-project",id:"guides/root-project",title:"Root-level project",description:"Coming from other repositories or task runner, you may be familiar with tasks available at the",source:"@site/docs/guides/root-project.mdx",sourceDirName:"guides",slug:"/guides/root-project",permalink:"/docs/guides/root-project",draft:!1,editUrl:"https://github.com/moonrepo/moon/tree/master/website/docs/guides/root-project.mdx",tags:[],version:"current",frontMatter:{title:"Root-level project"},sidebar:"guides",previous:{title:"Remote caching",permalink:"/docs/guides/remote-cache"},next:{title:"Sharing workspace configuration",permalink:"/docs/guides/sharing-config"}},l={},c=[{value:"Caveats",id:"caveats",level:2},{value:"Greedy inputs",id:"greedy-inputs",level:3},{value:"Inherited tasks",id:"inherited-tasks",level:3}],p={toc:c};function u(e){let{components:t,...n}=e;return(0,o.kt)("wrapper",(0,r.Z)({},p,n,{components:t,mdxType:"MDXLayout"}),(0,o.kt)("p",null,"Coming from other repositories or task runner, you may be familiar with tasks available at the\nrepository root, in which one-off, organization, maintenance, or process oriented tasks can be ran.\nmoon supports this through a concept known as a root-level project."),(0,o.kt)("p",null,"Begin by adding the root to ",(0,o.kt)("a",{parentName:"p",href:"../config/workspace#projects"},(0,o.kt)("inlineCode",{parentName:"a"},"projects"))," with a source value of ",(0,o.kt)("inlineCode",{parentName:"p"},"."),"\n(current directory relative from the workspace)."),(0,o.kt)("pre",null,(0,o.kt)("code",{parentName:"pre",className:"language-yaml",metastring:'title=".moon/workspace.yml"',title:'".moon/workspace.yml"'},"# As a map\nprojects:\n  root: '.'\n\n# As a list of globs\nprojects:\n  - '.'\n")),(0,o.kt)("blockquote",null,(0,o.kt)("p",{parentName:"blockquote"},"When using globs, the root project's name will be inferred from the repository folder name. Be\nwary of this as it can change based on what a developer has checked out as.")),(0,o.kt)("p",null,"Once added, create a ",(0,o.kt)("a",{parentName:"p",href:"../config/project"},(0,o.kt)("inlineCode",{parentName:"a"},"moon.yml"))," in the root of the repository. From here you\ncan define tasks that can be ran using this new root-level project name, for example,\n",(0,o.kt)("inlineCode",{parentName:"p"},"moon run root:<task>"),"."),(0,o.kt)("pre",null,(0,o.kt)("code",{parentName:"pre",className:"language-yaml",metastring:'title="moon.yml"',title:'"moon.yml"'},"tasks:\n    versionCheck:\n        command: 'yarn version check'\n        inputs: []\n        options:\n            cache: false\n")),(0,o.kt)("p",null,"And that's it, but there are a few caveats to be aware of..."),(0,o.kt)("h2",{id:"caveats"},"Caveats"),(0,o.kt)("h3",{id:"greedy-inputs"},"Greedy inputs"),(0,o.kt)("p",null,"Task ",(0,o.kt)("a",{parentName:"p",href:"../config/project#inputs"},(0,o.kt)("inlineCode",{parentName:"a"},"inputs"))," default to ",(0,o.kt)("inlineCode",{parentName:"p"},"**/*"),", which would result in root-level tasks\nscanning ",(0,o.kt)("em",{parentName:"p"},"all")," files in the repository. This will be a very expensive operation! We suggest\nrestricting inputs to a very succinct whitelist, or disabling inputs entirely."),(0,o.kt)("pre",null,(0,o.kt)("code",{parentName:"pre",className:"language-yaml",metastring:'title="moon.yml"',title:'"moon.yml"'},"tasks:\n    oneOff:\n        # ...\n        inputs: []\n")),(0,o.kt)("h3",{id:"inherited-tasks"},"Inherited tasks"),(0,o.kt)("p",null,"Because a root project is still a project in the workspace, it will inherit all tasks defined in\n",(0,o.kt)("a",{parentName:"p",href:"../config/tasks"},(0,o.kt)("inlineCode",{parentName:"a"},".moon/tasks.yml")),", which may be unexpected. To mitigate this, you can exclude\nsome or all of these tasks in the root config with\n",(0,o.kt)("a",{parentName:"p",href:"../config/project#inheritedtasks"},(0,o.kt)("inlineCode",{parentName:"a"},"workspace.inheritedTasks")),"."),(0,o.kt)("pre",null,(0,o.kt)("code",{parentName:"pre",className:"language-yaml",metastring:'title="moon.yml"',title:'"moon.yml"'},"workspace:\n    inheritedTasks:\n        include: []\n")))}u.isMDXComponent=!0}}]);