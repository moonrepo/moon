"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[655],{35281:(e,t,n)=>{n.r(t),n.d(t,{assets:()=>i,contentTitle:()=>l,default:()=>d,frontMatter:()=>o,metadata:()=>s,toc:()=>u});var a=n(24246),r=n(71670);n(33337),n(39798);const o={slug:"moon-v1.29",title:"moon v1.29 - Improved affected tracking, experimental Pkl configuration, and more",authors:["milesj"],tags:["affected","detection","tracker","project","task","config","pkl"],image:"./img/moon/v1.29.png"},l=void 0,s={permalink:"/blog/moon-v1.29",editUrl:"https://github.com/moonrepo/moon/tree/master/website/blog/2024-10-07_moon-v1.29.mdx",source:"@site/blog/2024-10-07_moon-v1.29.mdx",title:"moon v1.29 - Improved affected tracking, experimental Pkl configuration, and more",description:"In this release, we're excited to introduce an improved affected tracker and a new (but",date:"2024-10-07T00:00:00.000Z",tags:[{inline:!0,label:"affected",permalink:"/blog/tags/affected"},{inline:!0,label:"detection",permalink:"/blog/tags/detection"},{inline:!0,label:"tracker",permalink:"/blog/tags/tracker"},{inline:!0,label:"project",permalink:"/blog/tags/project"},{inline:!0,label:"task",permalink:"/blog/tags/task"},{inline:!0,label:"config",permalink:"/blog/tags/config"},{inline:!0,label:"pkl",permalink:"/blog/tags/pkl"}],readingTime:8.265,hasTruncateMarker:!0,authors:[{name:"Miles Johnson",title:"Founder, developer",url:"https://github.com/milesj",imageURL:"/img/authors/miles.jpg",key:"milesj"}],frontMatter:{slug:"moon-v1.29",title:"moon v1.29 - Improved affected tracking, experimental Pkl configuration, and more",authors:["milesj"],tags:["affected","detection","tracker","project","task","config","pkl"],image:"./img/moon/v1.29.png"},unlisted:!1,nextItem:{title:"moon v1.28 - Task presets, OS tasks, meta tokens, and more",permalink:"/blog/moon-v1.28"}},i={image:n(88410).Z,authorsImageUrls:[void 0]},u=[];function c(e){const t={p:"p",...(0,r.a)(),...e.components};return(0,a.jsx)(t.p,{children:"In this release, we're excited to introduce an improved affected tracker and a new (but\nexperimental) configuration format!"})}function d(e={}){const{wrapper:t}={...(0,r.a)(),...e.components};return t?(0,a.jsx)(t,{...e,children:(0,a.jsx)(c,{...e})}):c(e)}},39798:(e,t,n)=>{n.d(t,{Z:()=>l});n(27378);var a=n(40624);const r={tabItem:"tabItem_wHwb"};var o=n(24246);function l(e){let{children:t,hidden:n,className:l}=e;return(0,o.jsx)("div",{role:"tabpanel",className:(0,a.Z)(r.tabItem,l),hidden:n,children:t})}},33337:(e,t,n)=>{n.d(t,{Z:()=>p});var a=n(27378),r=n(40624),o=n(83457),l=n(35595),s=n(76457);const i={tabList:"tabList_J5MA",tabItem:"tabItem_l0OV"};var u=n(24246);function c(e){let{className:t,block:n,selectedValue:a,selectValue:l,tabValues:s}=e;const c=[],{blockElementScrollPositionUntilNextRender:d}=(0,o.o5)(),m=e=>{const t=e.currentTarget,n=c.indexOf(t),r=s[n].value;r!==a&&(d(t),l(r))},p=e=>{let t=null;switch(e.key){case"Enter":m(e);break;case"ArrowRight":{const n=c.indexOf(e.currentTarget)+1;t=c[n]??c[0];break}case"ArrowLeft":{const n=c.indexOf(e.currentTarget)-1;t=c[n]??c[c.length-1];break}}t?.focus()};return(0,u.jsx)("ul",{role:"tablist","aria-orientation":"horizontal",className:(0,r.Z)("tabs",{"tabs--block":n},t),children:s.map((e=>{let{value:t,label:n,attributes:o}=e;return(0,u.jsx)("li",{role:"tab",tabIndex:a===t?0:-1,"aria-selected":a===t,ref:e=>c.push(e),onKeyDown:p,onClick:m,...o,className:(0,r.Z)("tabs__item",i.tabItem,o?.className,{"tabs__item--active":a===t}),children:n??t},t)}))})}function d(e){let{lazy:t,children:n,selectedValue:r}=e;const o=(Array.isArray(n)?n:[n]).filter(Boolean);if(t){const e=o.find((e=>e.props.value===r));return e?(0,a.cloneElement)(e,{className:"margin-top--md"}):null}return(0,u.jsx)("div",{className:"margin-top--md",children:o.map(((e,t)=>(0,a.cloneElement)(e,{key:t,hidden:e.props.value!==r})))})}function m(e){const t=(0,l.Y)(e);return(0,u.jsxs)("div",{className:(0,r.Z)("tabs-container",i.tabList),children:[(0,u.jsx)(c,{...t,...e}),(0,u.jsx)(d,{...t,...e})]})}function p(e){const t=(0,s.Z)();return(0,u.jsx)(m,{...e,children:(0,l.h)(e.children)},String(t))}},35595:(e,t,n)=>{n.d(t,{Y:()=>p,h:()=>u});var a=n(27378),r=n(3620),o=n(9834),l=n(30654),s=n(70784),i=n(55643);function u(e){return a.Children.toArray(e).filter((e=>"\n"!==e)).map((e=>{if(!e||(0,a.isValidElement)(e)&&function(e){const{props:t}=e;return!!t&&"object"==typeof t&&"value"in t}(e))return e;throw new Error(`Docusaurus error: Bad <Tabs> child <${"string"==typeof e.type?e.type:e.type.name}>: all children of the <Tabs> component should be <TabItem>, and every <TabItem> should have a unique "value" prop.`)}))?.filter(Boolean)??[]}function c(e){const{values:t,children:n}=e;return(0,a.useMemo)((()=>{const e=t??function(e){return u(e).map((e=>{let{props:{value:t,label:n,attributes:a,default:r}}=e;return{value:t,label:n,attributes:a,default:r}}))}(n);return function(e){const t=(0,s.l)(e,((e,t)=>e.value===t.value));if(t.length>0)throw new Error(`Docusaurus error: Duplicate values "${t.map((e=>e.value)).join(", ")}" found in <Tabs>. Every value needs to be unique.`)}(e),e}),[t,n])}function d(e){let{value:t,tabValues:n}=e;return n.some((e=>e.value===t))}function m(e){let{queryString:t=!1,groupId:n}=e;const o=(0,r.k6)(),s=function(e){let{queryString:t=!1,groupId:n}=e;if("string"==typeof t)return t;if(!1===t)return null;if(!0===t&&!n)throw new Error('Docusaurus error: The <Tabs> component groupId prop is required if queryString=true, because this value is used as the search param name. You can also provide an explicit value such as queryString="my-search-param".');return n??null}({queryString:t,groupId:n});return[(0,l._X)(s),(0,a.useCallback)((e=>{if(!s)return;const t=new URLSearchParams(o.location.search);t.set(s,e),o.replace({...o.location,search:t.toString()})}),[s,o])]}function p(e){const{defaultValue:t,queryString:n=!1,groupId:r}=e,l=c(e),[s,u]=(0,a.useState)((()=>function(e){let{defaultValue:t,tabValues:n}=e;if(0===n.length)throw new Error("Docusaurus error: the <Tabs> component requires at least one <TabItem> children component");if(t){if(!d({value:t,tabValues:n}))throw new Error(`Docusaurus error: The <Tabs> has a defaultValue "${t}" but none of its children has the corresponding value. Available values are: ${n.map((e=>e.value)).join(", ")}. If you intend to show no default tab, use defaultValue={null} instead.`);return t}const a=n.find((e=>e.default))??n[0];if(!a)throw new Error("Unexpected error: 0 tabValues");return a.value}({defaultValue:t,tabValues:l}))),[p,f]=m({queryString:n,groupId:r}),[b,g]=function(e){let{groupId:t}=e;const n=function(e){return e?`docusaurus.tab.${e}`:null}(t),[r,o]=(0,i.Nk)(n);return[r,(0,a.useCallback)((e=>{n&&o.set(e)}),[n,o])]}({groupId:r}),h=(()=>{const e=p??b;return d({value:e,tabValues:l})?e:null})();(0,o.Z)((()=>{h&&u(h)}),[h]);return{selectedValue:s,selectValue:(0,a.useCallback)((e=>{if(!d({value:e,tabValues:l}))throw new Error(`Can't select invalid tab value=${e}`);u(e),f(e),g(e)}),[f,g,l]),tabValues:l}}},88410:(e,t,n)=>{n.d(t,{Z:()=>a});const a=n.p+"assets/images/v1.29-e00b51e586685d72b0ea1d5ba6c30b29.png"},71670:(e,t,n)=>{n.d(t,{Z:()=>s,a:()=>l});var a=n(27378);const r={},o=a.createContext(r);function l(e){const t=a.useContext(o);return a.useMemo((function(){return"function"==typeof e?e(t):{...t,...e}}),[t,e])}function s(e){let t;return t=e.disableParentContext?"function"==typeof e.components?e.components(r):e.components||r:l(e.components),a.createElement(o.Provider,{value:t},e.children)}}}]);