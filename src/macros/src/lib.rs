#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

use quote::quote;
use std::collections::HashSet;
use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::{parenthesized, parse_macro_input, FnArg, Ident, Token};

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
pub fn syscalls(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let Syscalls { syscalls } = parse_macro_input!(input as Syscalls);

    match syscalls_impl(syscalls) {
        Ok(tokens) => tokens,
        Err(_) => proc_macro::TokenStream::new(),
    }
}

fn syscalls_impl(syscalls: Vec<Syscall>) -> Result<proc_macro::TokenStream, ()> {
    let duplicates = find_syscall_name_duplicates(&syscalls);
    if duplicates.len() > 0 {
        for duplicate in duplicates {
            duplicate
                .span()
                .unwrap()
                .error(format!("duplicate syscall name `{}`", duplicate))
                .emit();
        }
        return Err(());
    }

    let userspace_functions = generate_userspace_functions(&syscalls)?;

    // let userspace_module = quote! {
    //     mod userspace {
    //         extern crate alloc;

    //         use alloc::vec::Vec;

    //         #(#userspace_functions)*
    //     }
    // };

    Ok(proc_macro::TokenStream::new())
}

fn generate_userspace_functions(syscalls: &[Syscall]) -> Result<Vec<proc_macro2::TokenStream>, ()> {
    let mut userspace_functions = Vec::new();

    for syscall in syscalls {
        match syscall.args.len() {
            1 => {
                let arg1 = &syscall.args[0];
                let name = &syscall.name;
                let id = syscall.id;
                userspace_functions.push(quote! {
                    pub fn #name(#arg1) {
                        // syscall_1(#id, #arg1);
                    }
                });
            }
            2 => {
                let arg1 = &syscall.args[0];
                let arg2 = &syscall.args[1];
                let name = &syscall.name;
                let id = syscall.id;
                userspace_functions.push(quote! {
                    pub fn #name(#arg1, #arg2) -> isize {
                        // syscall_2(#id, #arg1, #arg2)
                    }
                });
            }
            _ => panic!(
                "Unsupported number of arguments for syscall {}",
                syscall.name
            ),
        };
    }
    Ok(userspace_functions)
}

fn maybe_convert_reference(argument: &FnArg) -> Result<FnArg, ()> {
    match argument {
        FnArg::Receiver(receiver) => {
            receiver
                .span()
                .unwrap()
                .error("Argument self is not supported")
                .emit();
            Err(())
        }
        FnArg::Typed(_) => Ok(argument.clone()),
    }
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
