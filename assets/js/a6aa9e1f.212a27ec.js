"use strict";(self.webpackChunkwebsite=self.webpackChunkwebsite||[]).push([[93089],{24900:(e,t,a)=>{a.r(t),a.d(t,{default:()=>b});a(27378);var i=a(40624),n=a(50353),r=a(88676),s=a(75484),o=a(55228),l=a(20013),d=a(60505),c=a(2134),g=a(7092),u=a(84267),m=a(24246);function p(e){const t=(0,u.C)(e);return(0,m.jsx)(g.Z,{children:(0,m.jsx)("script",{type:"application/ld+json",children:JSON.stringify(t)})})}function h(e){const{metadata:t}=e,{siteConfig:{title:a}}=(0,n.default)(),{blogDescription:i,blogTitle:s,permalink:o}=t,l="/"===o?a:s;return(0,m.jsxs)(m.Fragment,{children:[(0,m.jsx)(r.d,{title:l,description:i}),(0,m.jsx)(d.Z,{tag:"blog_posts_list"})]})}function f(e){const{metadata:t,items:a,sidebar:i}=e;return(0,m.jsxs)(o.Z,{sidebar:i,children:[(0,m.jsx)(c.Z,{items:a}),(0,m.jsx)(l.Z,{metadata:t})]})}function b(e){return(0,m.jsxs)(r.FG,{className:(0,i.Z)(s.k.wrapper.blogPages,s.k.page.blogListPage),children:[(0,m.jsx)(h,{...e}),(0,m.jsx)(p,{...e}),(0,m.jsx)(f,{...e})]})}},20013:(e,t,a)=>{a.d(t,{Z:()=>s});a(27378);var i=a(99213),n=a(44022),r=a(24246);function s(e){const{metadata:t}=e,{previousPage:a,nextPage:s}=t;return(0,r.jsxs)("nav",{className:"pagination-nav","aria-label":(0,i.I)({id:"theme.blog.paginator.navAriaLabel",message:"Blog list page navigation",description:"The ARIA label for the blog pagination"}),children:[a&&(0,r.jsx)(n.Z,{permalink:a,title:(0,r.jsx)(i.Z,{id:"theme.blog.paginator.newerEntries",description:"The label used to navigate to the newer blog posts page (previous page)",children:"Newer Entries"})}),s&&(0,r.jsx)(n.Z,{permalink:s,title:(0,r.jsx)(i.Z,{id:"theme.blog.paginator.olderEntries",description:"The label used to navigate to the older blog posts page (next page)",children:"Older Entries"}),isNext:!0})]})}},2134:(e,t,a)=>{a.d(t,{Z:()=>s});a(27378);var i=a(70412),n=a(23952),r=a(24246);function s(e){let{items:t,component:a=n.Z}=e;return(0,r.jsx)(r.Fragment,{children:t.map((e=>{let{content:t}=e;return(0,r.jsx)(i.n,{content:t,children:(0,r.jsx)(a,{children:(0,r.jsx)(t,{})})},t.metadata.permalink)}))})}},84267:(e,t,a)=>{a.d(t,{C:()=>c,i:()=>g});var i=a(98948),n=a(50353),r=a(74909);var s=a(70412);const o=e=>new Date(e).toISOString();function l(e){const t=e.map(u);return{author:1===t.length?t[0]:t}}function d(e,t,a){return e?{image:m({imageUrl:t(e,{absolute:!0}),caption:`title image for the blog post: ${a}`})}:{}}function c(e){const{siteConfig:t}=(0,n.default)(),{withBaseUrl:a}=(0,i.C)(),{metadata:{blogDescription:r,blogTitle:s,permalink:c}}=e,g=`${t.url}${c}`;return{"@context":"https://schema.org","@type":"Blog","@id":g,mainEntityOfPage:g,headline:s,description:r,blogPost:e.items.map((e=>function(e,t,a){const{assets:i,frontMatter:n,metadata:r}=e,{date:s,title:c,description:g,lastUpdatedAt:u}=r,m=i.image??n.image,p=n.keywords??[],h=`${t.url}${r.permalink}`,f=u?o(u):void 0;return{"@type":"BlogPosting","@id":h,mainEntityOfPage:h,url:h,headline:c,name:c,description:g,datePublished:s,...f?{dateModified:f}:{},...l(r.authors),...d(m,a,c),...p?{keywords:p}:{}}}(e.content,t,a)))}}function g(){const e=function(){const e=(0,r.Z)(),t=e?.data?.blogMetadata;if(!t)throw new Error("useBlogMetadata() can't be called on the current route because the blog metadata could not be found in route context");return t}(),{assets:t,metadata:a}=(0,s.C)(),{siteConfig:c}=(0,n.default)(),{withBaseUrl:g}=(0,i.C)(),{date:u,title:m,description:p,frontMatter:h,lastUpdatedAt:f}=a,b=t.image??h.image,x=h.keywords??[],j=f?o(f):void 0,Z=`${c.url}${a.permalink}`;return{"@context":"https://schema.org","@type":"BlogPosting","@id":Z,mainEntityOfPage:Z,url:Z,headline:m,name:m,description:p,datePublished:u,...j?{dateModified:j}:{},...l(a.authors),...d(b,g,m),...x?{keywords:x}:{},isPartOf:{"@type":"Blog","@id":`${c.url}${e.blogBasePath}`,name:e.blogTitle}}}function u(e){return{"@type":"Person",...e.name?{name:e.name}:{},...e.title?{description:e.title}:{},...e.url?{url:e.url}:{},...e.email?{email:e.email}:{},...e.imageURL?{image:e.imageURL}:{}}}function m(e){let{imageUrl:t,caption:a}=e;return{"@type":"ImageObject","@id":t,url:t,contentUrl:t,caption:a}}},44022:(e,t,a)=>{a.d(t,{Z:()=>l});var i=a(40624),n=a(83469),r=a(31792),s=a(90728),o=a(24246);function l(e){let{permalink:t,title:a,isNext:l}=e;return(0,o.jsx)("div",{className:(0,i.Z)("flex-1",l?"text-right":"text-left"),children:(0,o.jsxs)(s.Z,{weight:"bold",to:t,children:[!l&&(0,o.jsx)(r.Z,{className:"mr-1 icon-previous",icon:n.A35}),a,l&&(0,o.jsx)(r.Z,{className:"ml-1 icon-next",icon:n._tD})]})})}}}]);