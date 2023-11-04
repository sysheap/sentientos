#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
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
    args: Vec<syn::FnArg>,
}

impl Parse for Syscalls {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut syscalls = Vec::<Syscall>::new();
        let mut next_syscall_id = 0;

        while !input.is_empty() {
            let syscall_name = input.parse::<Ident>()?;

            let args_content;
            parenthesized!(args_content in input);

            let args = args_content.parse_terminated(syn::FnArg::parse, Token![,])?;

            input.parse::<Token![;]>()?;

            syscalls.push(Syscall {
                id: next_syscall_id,
                name: syscall_name,
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
    check_for_duplicates_and_report_error(&syscalls)?;
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
        let ecall = generate_ecall(syscall.id, syscall_arguments)?;

        userspace_functions.push(quote! {
            #[allow(non_snake_case)]
            pub fn #syscall_name(#(#syscall_arguments),*) -> isize {
                #ecall
            }
        });
    }
    Ok(userspace_functions)
}

fn generate_ecall(
    syscall_number: usize,
    arguments: &[FnArg],
) -> Result<proc_macro2::TokenStream, ()> {
    match arguments.len() {
        1 => {
            let arg0 = cast_argument(&arguments[0])?;
            Ok(quote! {
                ecall_1(#syscall_number, #arg0)
            })
        }
        2 => {
            let arg0 = cast_argument(&arguments[0])?;
            let arg1 = cast_argument(&arguments[1])?;
            Ok(quote! {
                ecall_2(#syscall_number, #arg0, #arg1)
            })
        }
        _ => panic!("Not implemented yet"),
    }
}

fn cast_argument(argument: &FnArg) -> Result<TokenStream, ()> {
    let argument_name = format_ident!("{}", get_argument_name(argument));
    match get_argument_type(argument)? {
        ArgumentType::Reference => Ok(quote! { #argument_name as *const _ as usize }),
        ArgumentType::MutableReference => Ok(quote! { #argument_name as *const _ as usize }),
        ArgumentType::Value => Ok(quote! { #argument_name as usize }),
    }
}

enum ArgumentType {
    Reference,
    MutableReference,
    Value,
}

fn get_argument_type(argument: &FnArg) -> Result<ArgumentType, ()> {
    let result = match argument {
        FnArg::Typed(typed) => match *typed.ty.clone() {
            syn::Type::Reference(reference) => {
                if reference.mutability.is_some() {
                    Ok(ArgumentType::MutableReference)
                } else {
                    Ok(ArgumentType::Reference)
                }
            }
            syn::Type::Path(path) => {
                if path.path.segments.len() == 1 {
                    let segment = &path.path.segments[0];
                    if is_ident_value_type(&segment.ident) {
                        Ok(ArgumentType::Value)
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            }
            _ => Err(()),
        },
        _ => Err(()),
    };

    if result.is_err() {
        argument
            .span()
            .unwrap()
            .error(format!("unsupported argument type {:?}", argument))
            .emit();
    }

    result
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

fn is_ident_value_type(ident: &Ident) -> bool {
    let token_stream_type = ident.to_string();
    matches!(
        token_stream_type.as_str(),
        "u8" | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "char"
    )
}

fn check_for_duplicates_and_report_error(syscalls: &[Syscall]) -> Result<(), ()> {
    let duplicates = find_syscall_name_duplicates(syscalls);

    if duplicates.is_empty() {
        return Ok(());
    }

    for duplicate in duplicates {
        duplicate
            .span()
            .unwrap()
            .error(format!("duplicate syscall name `{}`", duplicate))
            .emit();
    }
    Err(())
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
