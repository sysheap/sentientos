#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

use std::collections::{HashMap, HashSet};

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::{parenthesized, parse_macro_input, Ident, Token};

struct Syscalls {
    syscalls: Vec<Syscall>,
}

struct Syscall {
    id: usize,
    name: syn::Ident,
    handler: syn::Ident,
    args: Vec<syn::FnArg>,
}

impl Parse for Syscalls {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut syscalls = Vec::<Syscall>::new();
        let mut next_syscall_id = 0;

        while !input.is_empty() {
            let syscall_name = input.parse::<Ident>()?;

            let handler_content;
            parenthesized!(handler_content in input);

            let handler = handler_content.parse::<Ident>()?;

            let args_content;
            parenthesized!(args_content in input);

            let args = args_content.parse_terminated(syn::FnArg::parse, Token![,])?;

            input.parse::<Token![;]>()?;

            syscalls.push(Syscall {
                id: next_syscall_id,
                name: syscall_name,
                handler,
                args: args.into_iter().collect(),
            });
            next_syscall_id += 1;
        }

        Ok(Self { syscalls })
    }
}

#[proc_macro]
pub fn syscalls(input: TokenStream) -> TokenStream {
    let Syscalls { syscalls } = parse_macro_input!(input as Syscalls);

    let duplicates = find_syscall_name_duplicates(&syscalls);

    if duplicates.len() > 0 {
        for duplicate in duplicates {
            duplicate
                .span()
                .unwrap()
                .error(format!("duplicate syscall name `{}`", duplicate))
                .emit();
        }
        return TokenStream::new();
    }

    let userspace_mod = quote! {
        mod userspace {

        }
    };

    TokenStream::new()
}

fn find_syscall_name_duplicates(syscalls: &[Syscall]) -> Vec<&Ident> {
    let mut duplicates = Vec::<&Ident>::new();
    let mut names = HashSet::<&Ident>::new();

    for syscall in syscalls {
        if names.contains(&syscall.name) {
            duplicates.push(&syscall.name);
        } else {
            names.insert(&syscall.name);
        }
    }

    duplicates
}
