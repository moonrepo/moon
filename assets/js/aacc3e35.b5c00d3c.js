"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[73527],{36739:e=>{e.exports=JSON.parse('{"permalink":"/blog/v0.24","editUrl":"https://github.com/moonrepo/moon/tree/master/website/blog/2023-02-13_v0.24.mdx","source":"@site/blog/2023-02-13_v0.24.mdx","title":"moon v0.24 - Remote caching, interactive tasks, query improvements, and more","description":"With this release, we\'ve polished our CLI experience and improved task interoperability.","date":"2023-02-13T00:00:00.000Z","tags":[{"inline":true,"label":"project","permalink":"/blog/tags/project"},{"inline":true,"label":"platform","permalink":"/blog/tags/platform"},{"inline":true,"label":"moonbase","permalink":"/blog/tags/moonbase"},{"inline":true,"label":"remote-cache","permalink":"/blog/tags/remote-cache"}],"readingTime":4.28,"hasTruncateMarker":true,"authors":[{"name":"Miles Johnson","title":"Founder, developer","url":"https://github.com/milesj","imageURL":"/img/authors/miles.jpg","key":"milesj","page":null},{"name":"James Pozdena","title":"Founder, developer","url":"https://github.com/jpoz","imageURL":"/img/authors/james.jpg","key":"jpoz","page":null}],"frontMatter":{"slug":"v0.24","title":"moon v0.24 - Remote caching, interactive tasks, query improvements, and more","authors":["milesj","jpoz"],"tags":["project","platform","moonbase","remote-cache"],"image":"./img/v0.24.png"},"unlisted":false,"prevItem":{"title":"moon v0.25 - Deno tier 2 support, CI insights, custom project languages, and more","permalink":"/blog/v0.25"},"nextItem":{"title":"Remote caching is now publicly available through moonbase","permalink":"/blog/moonbase"}}')},43023:(e,t,r)=>{r.d(t,{R:()=>l,x:()=>s});var a=r(63696);const n={},o=a.createContext(n);function l(e){const t=a.useContext(o);return a.useMemo((function(){return"function"==typeof e?e(t):{...t,...e}}),[t,e])}function s(e){let t;return t=e.disableParentContext?"function"==typeof e.components?e.components(n):e.components||n:l(e.components),a.createElement(o.Provider,{value:t},e.children)}},49198:(e,t,r)=>{r.r(t),r.d(t,{assets:()=>i,contentTitle:()=>s,default:()=>m,frontMatter:()=>l,metadata:()=>a,toc:()=>u});var a=r(36739),n=r(62540),o=r(43023);r(65457),r(97265),r(43067);const l={slug:"v0.24",title:"moon v0.24 - Remote caching, interactive tasks, query improvements, and more",authors:["milesj","jpoz"],tags:["project","platform","moonbase","remote-cache"],image:"./img/v0.24.png"},s=void 0,i={image:r(80544).A,authorsImageUrls:[void 0,void 0]},u=[];function c(e){const t={p:"p",...(0,o.R)(),...e.components};return(0,n.jsx)(t.p,{children:"With this release, we've polished our CLI experience and improved task interoperability."})}function m(e={}){const{wrapper:t}={...(0,o.R)(),...e.components};return t?(0,n.jsx)(t,{...e,children:(0,n.jsx)(c,{...e})}):c(e)}},65457:(e,t,r)=>{r.d(t,{A:()=>j});var a=r(63696),n=r(11750),o=r(93707),l=r(49519),s=r(83604),i=r(95196),u=r(76229),c=r(88030);function m(e){return a.Children.toArray(e).filter((e=>"\n"!==e)).map((e=>{if(!e||(0,a.isValidElement)(e)&&function(e){const{props:t}=e;return!!t&&"object"==typeof t&&"value"in t}(e))return e;throw new Error(`Docusaurus error: Bad <Tabs> child <${"string"==typeof e.type?e.type:e.type.name}>: all children of the <Tabs> component should be <TabItem>, and every <TabItem> should have a unique "value" prop.`)}))?.filter(Boolean)??[]}function p(e){const{values:t,children:r}=e;return(0,a.useMemo)((()=>{const e=t??function(e){return m(e).map((e=>{let{props:{value:t,label:r,attributes:a,default:n}}=e;return{value:t,label:r,attributes:a,default:n}}))}(r);return function(e){const t=(0,u.XI)(e,((e,t)=>e.value===t.value));if(t.length>0)throw new Error(`Docusaurus error: Duplicate values "${t.map((e=>e.value)).join(", ")}" found in <Tabs>. Every value needs to be unique.`)}(e),e}),[t,r])}function d(e){let{value:t,tabValues:r}=e;return r.some((e=>e.value===t))}function b(e){let{queryString:t=!1,groupId:r}=e;const n=(0,l.W6)(),o=function(e){let{queryString:t=!1,groupId:r}=e;if("string"==typeof t)return t;if(!1===t)return null;if(!0===t&&!r)throw new Error('Docusaurus error: The <Tabs> component groupId prop is required if queryString=true, because this value is used as the search param name. You can also provide an explicit value such as queryString="my-search-param".');return r??null}({queryString:t,groupId:r});return[(0,i.aZ)(o),(0,a.useCallback)((e=>{if(!o)return;const t=new URLSearchParams(n.location.search);t.set(o,e),n.replace({...n.location,search:t.toString()})}),[o,n])]}function h(e){const{defaultValue:t,queryString:r=!1,groupId:n}=e,o=p(e),[l,i]=(0,a.useState)((()=>function(e){let{defaultValue:t,tabValues:r}=e;if(0===r.length)throw new Error("Docusaurus error: the <Tabs> component requires at least one <TabItem> children component");if(t){if(!d({value:t,tabValues:r}))throw new Error(`Docusaurus error: The <Tabs> has a defaultValue "${t}" but none of its children has the corresponding value. Available values are: ${r.map((e=>e.value)).join(", ")}. If you intend to show no default tab, use defaultValue={null} instead.`);return t}const a=r.find((e=>e.default))??r[0];if(!a)throw new Error("Unexpected error: 0 tabValues");return a.value}({defaultValue:t,tabValues:o}))),[u,m]=b({queryString:r,groupId:n}),[h,g]=function(e){let{groupId:t}=e;const r=function(e){return e?`docusaurus.tab.${e}`:null}(t),[n,o]=(0,c.Dv)(r);return[n,(0,a.useCallback)((e=>{r&&o.set(e)}),[r,o])]}({groupId:n}),f=(()=>{const e=u??h;return d({value:e,tabValues:o})?e:null})();(0,s.A)((()=>{f&&i(f)}),[f]);return{selectedValue:l,selectValue:(0,a.useCallback)((e=>{if(!d({value:e,tabValues:o}))throw new Error(`Can't select invalid tab value=${e}`);i(e),m(e),g(e)}),[m,g,o]),tabValues:o}}var g=r(95200);const f={tabList:"tabList_J5MA",tabItem:"tabItem_l0OV"};var v=r(62540);function y(e){let{className:t,block:r,selectedValue:a,selectValue:l,tabValues:s}=e;const i=[],{blockElementScrollPositionUntilNextRender:u}=(0,o.a_)(),c=e=>{const t=e.currentTarget,r=i.indexOf(t),n=s[r].value;n!==a&&(u(t),l(n))},m=e=>{let t=null;switch(e.key){case"Enter":c(e);break;case"ArrowRight":{const r=i.indexOf(e.currentTarget)+1;t=i[r]??i[0];break}case"ArrowLeft":{const r=i.indexOf(e.currentTarget)-1;t=i[r]??i[i.length-1];break}}t?.focus()};return(0,v.jsx)("ul",{role:"tablist","aria-orientation":"horizontal",className:(0,n.A)("tabs",{"tabs--block":r},t),children:s.map((e=>{let{value:t,label:r,attributes:o}=e;return(0,v.jsx)("li",{role:"tab",tabIndex:a===t?0:-1,"aria-selected":a===t,ref:e=>{i.push(e)},onKeyDown:m,onClick:c,...o,className:(0,n.A)("tabs__item",f.tabItem,o?.className,{"tabs__item--active":a===t}),children:r??t},t)}))})}function k(e){let{lazy:t,children:r,selectedValue:o}=e;const l=(Array.isArray(r)?r:[r]).filter(Boolean);if(t){const e=l.find((e=>e.props.value===o));return e?(0,a.cloneElement)(e,{className:(0,n.A)("margin-top--md",e.props.className)}):null}return(0,v.jsx)("div",{className:"margin-top--md",children:l.map(((e,t)=>(0,a.cloneElement)(e,{key:t,hidden:e.props.value!==o})))})}function w(e){const t=h(e);return(0,v.jsxs)("div",{className:(0,n.A)("tabs-container",f.tabList),children:[(0,v.jsx)(y,{...t,...e}),(0,v.jsx)(k,{...t,...e})]})}function j(e){const t=(0,g.A)();return(0,v.jsx)(w,{...e,children:m(e.children)},String(t))}},80544:(e,t,r)=>{r.d(t,{A:()=>a});const a=r.p+"assets/images/v0.24-0e225eaeb8b3c60cc26907770c589000.png"},97265:(e,t,r)=>{r.d(t,{A:()=>l});r(63696);var a=r(11750);const n={tabItem:"tabItem_wHwb"};var o=r(62540);function l(e){let{children:t,hidden:r,className:l}=e;return(0,o.jsx)("div",{role:"tabpanel",className:(0,a.A)(n.tabItem,l),hidden:r,children:t})}}}]);