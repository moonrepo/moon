use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_attribute]
pub fn with_shared_exec_props(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let shared_fields = [
        // COMMON
        quote! {
            #[arg(
                long,
                short = 'f',
                env = "MOON_FORCE",
                help = "Force run and bypass cache, ignore changed files, and skip affected checks"
            )]
            pub force: bool
        },
        quote! {
            #[arg(long, short = 'i', help = "Run the pipeline and tasks interactively")]
            pub interactive: bool
        },
        quote! {
            #[arg(
                long,
                env = "MOON_NO_ACTIONS",
                help = "Run the pipeline without sync and setup related actions",
                help_heading = super::HEADING_WORKFLOW,
            )]
            pub no_actions: bool
        },
        quote! {
            #[arg(
                long,
                short = 's',
                env = "MOON_SUMMARY",
                help = "Print a summary of all actions that were ran in the pipeline"
            )]
            pub summary: Option<Option<crate::app_options::SummaryLevel>>
        },
        // AFFECTED
        quote! {
            #[arg(
                long,
                env = "MOON_AFFECTED",
                help = "Only run tasks if affected by changed files",
                help_heading = super::HEADING_AFFECTED,
                group = "affected-args"
            )]
            pub affected: bool
        },
        quote! {
             #[arg(
                long,
                env = "MOON_BASE",
                help = "Base branch, commit, or revision to compare against",
                help_heading = super::HEADING_AFFECTED,
                requires = "affected-args",
            )]
            pub base: Option<String>
        },
        quote! {
            #[arg(
                long,
                env = "MOON_HEAD",
                help = "Current branch, commit, or revision to compare with",
                help_heading = super::HEADING_AFFECTED,
                requires = "affected-args",
            )]
            pub head: Option<String>
        },
        quote! {
            #[arg(
                long,
                help = "Filter changed files based on a changed status",
                help_heading = super::HEADING_AFFECTED,
                requires = "affected-args",
            )]
            pub status: Vec<moon_vcs::ChangedStatus>
        },
        quote! {
            #[arg(
                long,
                help = "Accept changed files from stdin for affected checks",
                help_heading = super::HEADING_AFFECTED,
                requires = "affected-args",
            )]
            pub stdin: bool
        },
        // GRAPH
        quote! {
            #[arg(
                long,
                visible_alias = "dependents",
                help = "Control the depth of downstream dependents",
                help_heading = super::HEADING_GRAPH,
            )]
            pub downstream: Option<DownstreamScope>
        },
        quote! {
            #[arg(
                long,
                visible_alias = "dependencies",
                help = "Control the depth of upstream dependencies",
                help_heading = super::HEADING_GRAPH,
            )]
            pub upstream: Option<UpstreamScope>
        },
        // PARALLELISM
        quote! {
            #[arg(
                long,
                env = "MOON_JOB",
                help = "Index of the current job",
                help_heading = super::HEADING_PARALLELISM,
            )]
            pub job: Option<usize>
        },
        quote! {
            #[arg(
                long,
                env = "MOON_JOB_TOTAL",
                help = "Total amount of jobs to run",
                help_heading = super::HEADING_PARALLELISM,
            )]
            pub job_total: Option<usize>
        },
    ]
    .map(|tokens| syn::Field::parse_named.parse2(tokens).unwrap());

    let mut ast = parse_macro_input!(item as DeriveInput);
    let struct_name = &ast.ident;

    match &mut ast.data {
        syn::Data::Struct(struct_data) => {
            let mut into_rows = vec![];

            for field in &shared_fields {
                let key = field.ident.as_ref().unwrap();

                into_rows.push(quote! {
                    #key: self.#key,
                });
            }

            if let syn::Fields::Named(fields) = &mut struct_data.fields {
                fields.named.extend(shared_fields);
            }

            quote! {
                #ast

                impl #struct_name {
                    pub fn into_exec_args(self) -> ExecArgs {
                        ExecArgs {
                            #(#into_rows)*
                            ..Default::default()
                        }
                    }
                }
            }
            .into()
        }
        _ => panic!("`with_shared_exec_props` can only be used with structs!"),
    }
}
