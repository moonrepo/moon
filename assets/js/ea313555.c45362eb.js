"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[77937],{13971:(e,n,s)=>{s.r(n),s.d(n,{assets:()=>l,contentTitle:()=>r,default:()=>d,frontMatter:()=>t,metadata:()=>a,toc:()=>c});var o=s(24246),i=s(71670);const t={title:"FAQ"},r=void 0,a={id:"faq",title:"FAQ",description:"General",source:"@site/docs/faq.mdx",sourceDirName:".",slug:"/faq",permalink:"/docs/faq",draft:!1,unlisted:!1,editUrl:"https://github.com/moonrepo/moon/tree/master/website/docs/faq.mdx",tags:[],version:"current",frontMatter:{title:"FAQ"},sidebar:"docs",previous:{title:"Terminology",permalink:"/docs/terminology"}},l={},c=[{value:"General",id:"general",level:2},{value:"Where did the name &quot;moon&quot; come from?",id:"where-did-the-name-moon-come-from",level:3},{value:"Will moon support other languages?",id:"will-moon-support-other-languages",level:3},{value:"Will moon support continuous deployment?",id:"will-moon-support-continuous-deployment",level:3},{value:"What should be considered the &quot;source of truth&quot;?",id:"what-should-be-considered-the-source-of-truth",level:3},{value:"How to stop moon formatting JSON and YAML files?",id:"how-to-stop-moon-formatting-json-and-yaml-files",level:3},{value:"Projects &amp; tasks",id:"projects--tasks",level:2},{value:"How to pipe or redirect tasks?",id:"how-to-pipe-or-redirect-tasks",level:3},{value:"How to run multiple commands within a task?",id:"how-to-run-multiple-commands-within-a-task",level:3},{value:"How to run tasks in a shell?",id:"how-to-run-tasks-in-a-shell",level:3},{value:"Can we run other languages?",id:"can-we-run-other-languages",level:3},{value:"JavaScript ecosystem",id:"javascript-ecosystem",level:2},{value:"Can we use <code>package.json</code> scripts?",id:"can-we-use-packagejson-scripts",level:3},{value:"Can moon version/publish packages?",id:"can-moon-versionpublish-packages",level:3},{value:"Why is npm/pnpm/yarn install running twice when running a task?",id:"why-is-npmpnpmyarn-install-running-twice-when-running-a-task",level:3},{value:"Troubleshooting",id:"troubleshooting",level:2},{value:"How to resolve the &quot;version &#39;GLIBC_X.XX&#39; not found&quot; error?",id:"how-to-resolve-the-version-glibc_xxx-not-found-error",level:3}];function h(e){const n={a:"a",admonition:"admonition",code:"code",h2:"h2",h3:"h3",li:"li",p:"p",pre:"pre",strong:"strong",ul:"ul",...(0,i.a)(),...e.components};return(0,o.jsxs)(o.Fragment,{children:[(0,o.jsx)(n.h2,{id:"general",children:"General"}),"\n",(0,o.jsx)(n.h3,{id:"where-did-the-name-moon-come-from",children:'Where did the name "moon" come from?'}),"\n",(0,o.jsx)(n.p,{children:"The first incarnation of the name was a misspelling of monorepo (= moonrepo). This is where the\ndomain moonrepo.dev came from, and our official company, moonrepo, Inc."}),"\n",(0,o.jsx)(n.p,{children:"However, moonrepo is quite a long name with many syllables, and as someone who prefers short 1\nsyllable words, moon was perfect. The word moon also has great symmetry, as you can see in our logo!"}),"\n",(0,o.jsxs)(n.p,{children:["But that's not all... moon is also an acronym. It originally stood for ",(0,o.jsx)(n.strong,{children:"m"}),"onorepo,\n",(0,o.jsx)(n.strong,{children:"o"}),"rganization, ",(0,o.jsx)(n.strong,{children:"o"}),"rchestration, and ",(0,o.jsx)(n.strong,{children:"n"}),"otification tool. But since moon can also be used for\npolyrepos, we replaced monorepo with ",(0,o.jsx)(n.strong,{children:"m"}),"anagement (as shown on the homepage). This is a great\nacronym, as it embraces what moon is trying to solve:"]}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.strong,{children:"M"}),"anage repos, projects, and tasks with ease."]}),"\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.strong,{children:"O"}),"rganize projects and the repo to scale."]}),"\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.strong,{children:"O"}),"rchestrate tasks as efficiently as possible."]}),"\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.strong,{children:"N"}),"otify developers and systems about important events."]}),"\n"]}),"\n",(0,o.jsx)(n.h3,{id:"will-moon-support-other-languages",children:"Will moon support other languages?"}),"\n",(0,o.jsxs)(n.p,{children:["Yes! Although we're focusing right now on the web ecosystem (Node.js, Rust, Go, PHP, Python, etc),\nwe've designed moon to be language agnostic and easily pluggable in the future. View our\n",(0,o.jsx)(n.a,{href:"/docs#supported-languages",children:"supported languages for more information"}),"."]}),"\n",(0,o.jsx)(n.h3,{id:"will-moon-support-continuous-deployment",children:"Will moon support continuous deployment?"}),"\n",(0,o.jsx)(n.p,{children:"Yes! We plan to integrate CD with the current build and CI system, but we are focusing on the latter\n2 for the time being. Why not start using moon today so that you can easily adopt CD when it's\nready?"}),"\n",(0,o.jsx)(n.h3,{id:"what-should-be-considered-the-source-of-truth",children:'What should be considered the "source of truth"?'}),"\n",(0,o.jsxs)(n.p,{children:["If you're a frontend developer, you'll assume that a ",(0,o.jsx)(n.code,{children:"package.json"})," is the source of truth for a\nproject, as it defines scripts, dependencies, and repo-local relations. While true, this breaks down\nwith additional tooling, like TypeScript project references, as now you must maintain\n",(0,o.jsx)(n.code,{children:"tsconfig.json"})," as well as ",(0,o.jsx)(n.code,{children:"package.json"}),". The risk of these falling out of sync is high."]}),"\n",(0,o.jsxs)(n.p,{children:["This problem is further exacerbated by more tooling, or additional programming languages. What if\nyour frontend project is dependent on a backend project? This isn't easily modeled in\n",(0,o.jsx)(n.code,{children:"package.json"}),". What if the backend project needs to be built and ran before running the frontend\nproject? Again, while not impossible, it's quite cumbersome to model in ",(0,o.jsx)(n.code,{children:"package.json"})," scripts. So\non and so forth."]}),"\n",(0,o.jsxs)(n.p,{children:["moon aims to solve this with a different approach, by standardizing all projects in the workspace on\n",(0,o.jsx)(n.a,{href:"./config/project",children:(0,o.jsx)(n.code,{children:"moon.yml"})}),". With this, the ",(0,o.jsx)(n.code,{children:"moon.yml"})," is the source of truth for each project,\nand provides us with the following:"]}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsx)(n.li,{children:"The configuration is language agnostic. All projects are configured in a similar manner."}),"\n",(0,o.jsx)(n.li,{children:"Tasks can reference other tasks easily. For example, npm scripts referencing rake tasks, and vice\nverse, is a non-ideal experience."}),"\n",(0,o.jsxs)(n.li,{children:["Dependencies defined with ",(0,o.jsx)(n.a,{href:"./config/project#dependson",children:(0,o.jsx)(n.code,{children:"dependsOn"})})," use moon project names, and\nnot language specific semantics. This field also easily populates the dependency/project graphs."]}),"\n",(0,o.jsxs)(n.li,{children:["For JavaScript projects:","\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"package.json"})," dependencies (via ",(0,o.jsx)(n.code,{children:"dependsOn"}),") are kept in sync when\n",(0,o.jsx)(n.a,{href:"./config/toolchain#syncprojectworkspacedependencies",children:(0,o.jsx)(n.code,{children:"node.syncProjectWorkspaceDependencies"})}),"\nis enabled."]}),"\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.code,{children:"tsconfig.json"})," project references (via ",(0,o.jsx)(n.code,{children:"dependsOn"}),") are kept in sync when\n",(0,o.jsx)(n.a,{href:"./config/toolchain#syncprojectreferences",children:(0,o.jsx)(n.code,{children:"typescript.syncProjectReferences"})})," is enabled."]}),"\n"]}),"\n"]}),"\n"]}),"\n",(0,o.jsx)(n.p,{children:"By using moon as the source of truth, we can ensure a healthy repository, by accurately keeping\neverything in sync, and modifying project/language configuration to operate effectively."}),"\n",(0,o.jsx)(n.admonition,{type:"info",children:(0,o.jsxs)(n.p,{children:["With all that being said, moon supports\n",(0,o.jsx)(n.a,{href:"./concepts/project#dependencies",children:"implicit dependency scanning"}),", if you'd prefer to continue\nutilizing language specific functionality, instead of migrating entirely to moon."]})}),"\n",(0,o.jsx)(n.h3,{id:"how-to-stop-moon-formatting-json-and-yaml-files",children:"How to stop moon formatting JSON and YAML files?"}),"\n",(0,o.jsxs)(n.p,{children:["To ensure a healthy repository state, moon constantly modifies JSON and YAML files, specifically\n",(0,o.jsx)(n.code,{children:"package.json"})," and ",(0,o.jsx)(n.code,{children:"tsconfig.json"}),". This may result in a different formatting style in regards to\nindentation. While there is no way to stop or turn off this functionality, we respect\n",(0,o.jsx)(n.a,{href:"https://editorconfig.org/",children:"EditorConfig"})," during this process."]}),"\n",(0,o.jsxs)(n.p,{children:["Create a root ",(0,o.jsx)(n.code,{children:".editorconfig"})," file to enforce a consistent syntax."]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-ini",metastring:'title=".editorconfig"',children:"[*.{json,yaml,yml}]\nindent_style = space\nindent_size = 4\n"})}),"\n",(0,o.jsx)(n.h2,{id:"projects--tasks",children:"Projects & tasks"}),"\n",(0,o.jsx)(n.h3,{id:"how-to-pipe-or-redirect-tasks",children:"How to pipe or redirect tasks?"}),"\n",(0,o.jsxs)(n.p,{children:["Piping (",(0,o.jsx)(n.code,{children:"|"}),") or redirecting (",(0,o.jsx)(n.code,{children:">"}),") the output of one moon task to another moon task, whether via\nstdin or through ",(0,o.jsx)(n.code,{children:"inputs"}),", is not possible within our pipeline (task runner) directly."]}),"\n",(0,o.jsxs)(n.p,{children:["However, we do support this functionality on the command line, or within a task itself, using the\n",(0,o.jsx)(n.a,{href:"./config/project#script",children:(0,o.jsx)(n.code,{children:"script"})})," setting."]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  pipe:\n    script: 'gen-json | jq ...'\n"})}),"\n",(0,o.jsx)(n.p,{children:"Alternativaly, you can wrap this script in something like a Bash file, and execute that instead."}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-bash",metastring:'title="scripts/pipe.sh"',children:"#!/usr/bin/env bash\ngen-json | jq ...\n"})}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  pipe:\n    command: 'bash ./scripts/pipe.sh'\n"})}),"\n",(0,o.jsx)(n.h3,{id:"how-to-run-multiple-commands-within-a-task",children:"How to run multiple commands within a task?"}),"\n",(0,o.jsxs)(n.p,{children:["Only ",(0,o.jsx)(n.a,{href:"./config/project#script",children:(0,o.jsx)(n.code,{children:"script"})})," based tasks can run multiple commands via ",(0,o.jsx)(n.code,{children:"&&"})," or ",(0,o.jsx)(n.code,{children:";"}),"\nsyntax. This is possible as we execute the entire script within a shell, and not directly with the\ntoolchain."]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  multiple:\n    script: 'mkdir test && cd test && do-something'\n"})}),"\n",(0,o.jsx)(n.h3,{id:"how-to-run-tasks-in-a-shell",children:"How to run tasks in a shell?"}),"\n",(0,o.jsxs)(n.p,{children:["By default, all tasks run in a shell, based on the task's ",(0,o.jsx)(n.a,{href:"./config/project#shell",children:(0,o.jsx)(n.code,{children:"shell"})})," option,\nas demonstrated below:"]}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  # Runs in a shell\n  global:\n    command: 'some-command-on-path'\n\n  # Custom shells\n  unix:\n    command: 'bash -c some-command'\n    options:\n      shell: false\n  windows:\n    command: 'pwsh.exe -c some-command'\n    options:\n      shell: false\n"})}),"\n",(0,o.jsx)(n.h3,{id:"can-we-run-other-languages",children:"Can we run other languages?"}),"\n",(0,o.jsxs)(n.p,{children:["Yes! Although our toolchain only supports a few languages at this time, you can still run other\nlanguages within tasks by setting their ",(0,o.jsx)(n.a,{href:"./config/project#platform-1",children:(0,o.jsx)(n.code,{children:"platform"})}),' to "system".\nSystem tasks are an escape hatch that will use any command available on the current machine.']}),"\n",(0,o.jsx)(n.pre,{children:(0,o.jsx)(n.code,{className:"language-yaml",metastring:'title="moon.yml"',children:"tasks:\n  # Ruby\n  lint:\n    command: 'rubocop'\n    platform: 'system'\n  # PHP\n  test:\n    command: 'phpunit tests'\n    platform: 'system'\n"})}),"\n",(0,o.jsx)(n.p,{children:"However, because these languages are not supported directly within our toolchain, they will not\nreceive the benefits of the toolchain. Some of which are:"}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsx)(n.li,{children:"Automatic installation of the language. System tasks expect the command to already exist in the\nenvironment, which requires the user to manually install them."}),"\n",(0,o.jsx)(n.li,{children:"Consistent language and dependency manager versions across all machines."}),"\n",(0,o.jsx)(n.li,{children:"Built-in cpu and heap profiling (language specific)."}),"\n",(0,o.jsx)(n.li,{children:"Automatic dependency installs when the lockfile changes."}),"\n",(0,o.jsx)(n.li,{children:"And many more."}),"\n"]}),"\n",(0,o.jsx)(n.h2,{id:"javascript-ecosystem",children:"JavaScript ecosystem"}),"\n",(0,o.jsxs)(n.h3,{id:"can-we-use-packagejson-scripts",children:["Can we use ",(0,o.jsx)(n.code,{children:"package.json"})," scripts?"]}),"\n",(0,o.jsxs)(n.p,{children:["We encourage everyone to define tasks in a ",(0,o.jsx)(n.a,{href:"./config/project#tasks",children:(0,o.jsx)(n.code,{children:"moon.yml"})})," file, as it allows\nfor additional metadata like ",(0,o.jsx)(n.code,{children:"inputs"}),", ",(0,o.jsx)(n.code,{children:"outputs"}),", ",(0,o.jsx)(n.code,{children:"options"}),", and more. However, if you'd like to\nkeep using ",(0,o.jsx)(n.code,{children:"package.json"})," scripts, enable the\n",(0,o.jsx)(n.a,{href:"./config/toolchain#infertasksfromscripts",children:(0,o.jsx)(n.code,{children:"node.inferTasksFromScripts"})})," setting."]}),"\n",(0,o.jsxs)(n.p,{children:["View the ",(0,o.jsx)(n.a,{href:"./migrate-to-moon",children:"official documentation"})," for more information on this approach,\nincluding risks, disadvantages, and caveats."]}),"\n",(0,o.jsx)(n.h3,{id:"can-moon-versionpublish-packages",children:"Can moon version/publish packages?"}),"\n",(0,o.jsx)(n.p,{children:"At this time, no, as we're focusing on the build and test aspect of development. With that being\nsaid, this is something we'd like to support first-class in the future, but until then, we suggest\nthe following popular tools:"}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:[(0,o.jsx)(n.a,{href:"https://yarnpkg.com/features/release-workflow",children:"Yarn releases"})," (requires >= v2)"]}),"\n",(0,o.jsx)(n.li,{children:(0,o.jsx)(n.a,{href:"https://github.com/changesets/changesets",children:"Changesets"})}),"\n",(0,o.jsx)(n.li,{children:(0,o.jsx)(n.a,{href:"https://github.com/lerna/lerna",children:"Lerna"})}),"\n"]}),"\n",(0,o.jsx)(n.h3,{id:"why-is-npmpnpmyarn-install-running-twice-when-running-a-task",children:"Why is npm/pnpm/yarn install running twice when running a task?"}),"\n",(0,o.jsxs)(n.p,{children:["moon will automatically install dependencies in a project or in the workspace root (when using\npackage workspaces) when the lockfile or ",(0,o.jsx)(n.code,{children:"package.json"})," has been modified since the last time the\ninstall ran. If you are running a task and multiple installs are occurring (and it's causing\nissues), it can mean 1 of 2 things:"]}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:["If you are using package workspaces, then 1 of the project's triggering the install is not listed\nwithin the ",(0,o.jsx)(n.code,{children:"workspaces"})," field in the root ",(0,o.jsx)(n.code,{children:"package.json"})," (for npm and yarn), or in\n",(0,o.jsx)(n.code,{children:"pnpm-workspace.yml"})," (for pnpm)."]}),"\n",(0,o.jsx)(n.li,{children:"If the install is triggering in a non-JavaScript related project, then this project is incorrectly\nlisted as a package workspace."}),"\n",(0,o.jsx)(n.li,{children:"If you don't want a package included in the workspace, but do want to install its dependencies,\nthen it'll need its own lockfile."}),"\n"]}),"\n",(0,o.jsx)(n.h2,{id:"troubleshooting",children:"Troubleshooting"}),"\n",(0,o.jsx)(n.h3,{id:"how-to-resolve-the-version-glibc_xxx-not-found-error",children:"How to resolve the \"version 'GLIBC_X.XX' not found\" error?"}),"\n",(0,o.jsx)(n.p,{children:"This is typically caused by running moon in an old environment, like Ubuntu 18, and the minimum\nrequired libc doesn't exist or is too old. Since moon is Rust based, we're unable to support all\nenvironments and versions perpetually, and will only support relatively modern environments."}),"\n",(0,o.jsx)(n.p,{children:"There's not an easy fix to this problem, but there are a few potential solutions, from easiest to\nhardest:"}),"\n",(0,o.jsxs)(n.ul,{children:["\n",(0,o.jsxs)(n.li,{children:["Run moon in a Docker container/image that has the correct environment and libs. For example, the\n",(0,o.jsx)(n.code,{children:"node:latest"})," image."]}),"\n",(0,o.jsx)(n.li,{children:"Upgrade the enviroment to a newer one. For example, Ubuntu 18 -> 22."}),"\n",(0,o.jsxs)(n.li,{children:["Try and install a newer libc\n(",(0,o.jsx)(n.a,{href:"https://stackoverflow.com/questions/72513993/how-install-glibc-2-29-or-higher-in-ubuntu-18-04",children:"more information"}),")."]}),"\n"]}),"\n",(0,o.jsxs)(n.p,{children:["For more information on this problem as a whole,\n",(0,o.jsx)(n.a,{href:"https://kobzol.github.io/rust/ci/2021/05/07/building-rust-binaries-in-ci-that-work-with-older-glibc.html",children:"refer to this in-depth article"}),"."]})]})}function d(e={}){const{wrapper:n}={...(0,i.a)(),...e.components};return n?(0,o.jsx)(n,{...e,children:(0,o.jsx)(h,{...e})}):h(e)}},71670:(e,n,s)=>{s.d(n,{Z:()=>a,a:()=>r});var o=s(27378);const i={},t=o.createContext(i);function r(e){const n=o.useContext(t);return o.useMemo((function(){return"function"==typeof e?e(n):{...n,...e}}),[n,e])}function a(e){let n;return n=e.disableParentContext?"function"==typeof e.components?e.components(i):e.components||i:r(e.components),o.createElement(t.Provider,{value:n},e.children)}}}]);