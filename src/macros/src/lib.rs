#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::parse::Parse;
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
    let userspace_module = generate_userspace_module(userspace_functions);

    Ok(proc_macro::TokenStream::from(userspace_module))
}

fn generate_userspace_module(userspace_functions: Vec<TokenStream>) -> TokenStream {
    quote! {
        pub mod userspace {
            extern crate alloc;

            use alloc::vec::Vec;
            use core::arch::asm;

            fn ecall_1(nr: usize, arg0: usize) -> isize {
                let ret: isize;
                unsafe {
                    asm!("ecall",
                        in("a7") nr,
                        in("a0") arg0,
                        lateout("a0") ret,
                    );
                }
                ret
            }

            fn ecall_2(nr: usize, arg0: usize, arg1: usize) -> isize {
                let ret: isize;
                unsafe {
                    asm!("ecall",
                        in("a7") nr,
                        in("a0") arg0,
                        in("a1") arg1,
                        lateout("a0") ret,
                    );
                }
                ret
            }

            #(#userspace_functions)*
        }
    }
}

fn generate_userspace_functions(syscalls: &[Syscall]) -> Result<Vec<proc_macro2::TokenStream>, ()> {
    let mut userspace_functions = Vec::new();

    for syscall in syscalls {
        let syscall_name = &syscall.name;
        let syscall_arguments = &syscall.args;
        let ecall = generate_ecall(syscall.id, &syscall_arguments);

        userspace_functions.push(quote! {
            #[allow(non_snake_case)]
            pub fn #syscall_name(#(#syscall_arguments),*) -> isize {
                #ecall
            }
        });
    }
    Ok(userspace_functions)
}

fn generate_ecall(syscall_number: usize, arguments: &[FnArg]) -> proc_macro2::TokenStream {
    match arguments.len() {
        1 => {
            let arg0 = cast_argument(&arguments[0]);
            quote! {
                ecall_1(#syscall_number, #arg0)
            }
        }
        2 => {
            let arg0 = cast_argument(&arguments[0]);
            let arg1 = cast_argument(&arguments[1]);
            quote! {
                ecall_2(#syscall_number, #arg0, #arg1)
            }
        }
        _ => panic!("Not implemented yet"),
    }
}

fn cast_argument(argument: &FnArg) -> TokenStream {
    let argument_name = format_ident!("{}", get_argument_name(argument));
    match get_argument_type(argument) {
        ArgumentType::Reference => quote! { #argument_name as *const _ as usize },
        ArgumentType::MutableReference => quote! { #argument_name as *const _ as usize },
        ArgumentType::Value => quote! { #argument_name as usize },
    }
}

enum ArgumentType {
    Reference,
    MutableReference,
    Value,
}

fn get_argument_type(argument: &FnArg) -> ArgumentType {
    match argument {
        FnArg::Typed(typed) => match *typed.ty.clone() {
            syn::Type::Reference(reference) => {
                if reference.mutability.is_some() {
                    ArgumentType::MutableReference
                } else {
                    ArgumentType::Reference
                }
            }
            syn::Type::Path(path) => {
                if path.path.segments.len() == 1 {
                    let segment = &path.path.segments[0];
                    if is_tokenstream_value_type(&segment.ident) {
                        ArgumentType::Value
                    } else {
                        panic!("Cannot get type of argument {:?}", argument)
                    }
                } else {
                    panic!("Cannot get type of argument {:?}", argument)
                }
            }
            _ => panic!("Cannot get type of argument {:?}", argument),
        },
        _ => panic!("Cannot get type of argument {:?}", argument),
    }
}

fn get_argument_name(argument: &FnArg) -> String {
    match argument {
        FnArg::Receiver(_) => "self".into(),
        FnArg::Typed(typed) => match *typed.pat.clone() {
            syn::Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
            _ => panic!("Cannot get name of argument {:?}", argument),
        },
    }
}

fn is_tokenstream_value_type(ident: &Ident) -> bool {
    let token_stream_type = ident.to_string();
    match token_stream_type.as_str() {
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128"
        | "char" => true,
        _ => false,
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
