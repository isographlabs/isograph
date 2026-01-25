use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, ItemStruct, Type, Visibility, parse_macro_input, spanned::Spanned};

pub(crate) fn db_macro(item: TokenStream) -> TokenStream {
    struct TrackedField {
        field_ident: syn::Ident,
        field_ty: Type,
    }

    let input = parse_macro_input!(item as ItemStruct);

    let fields = match &input.fields {
        syn::Fields::Named(named) => named,
        _ => {
            return syn::Error::new_spanned(&input.fields, "#[db] requires named fields")
                .to_compile_error()
                .into();
        }
    };

    let struct_ident = input.ident.clone();
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let mut tracked = Vec::new();
    let mut errors = Vec::new();

    for f in fields.named.iter() {
        let field_ident = f.ident.as_ref().unwrap();
        if has_attr(&f.attrs, "tracked") {
            if !matches!(f.vis, Visibility::Inherited) {
                errors.push(
                    syn::Error::new(
                        f.vis.span(),
                        "fields marked `#[tracked]` must be private (remove any `pub` visibility)",
                    )
                    .to_compile_error(),
                );
            }
            tracked.push(TrackedField {
                field_ident: field_ident.clone(),
                field_ty: f.ty.clone(),
            });
        }
    }

    if !errors.is_empty() {
        return quote! { #(#errors)* }.into();
    }

    let mut struct_impl = Vec::new();
    let mut aux_struct_defs = Vec::new();

    for TrackedField {
        field_ident,
        field_ty,
    } in tracked
    {
        let counter_ident =
            format_ident!("{}Counter", field_ident.to_string().to_case(Case::Pascal));
        let proj_ident = format_ident!(
            "{}_PROJECTOR",
            field_ident.to_string().to_case(Case::UpperSnake)
        );
        let proj_ident_mut = format_ident!(
            "{}_PROJECTOR_MUT",
            field_ident.to_string().to_case(Case::UpperSnake)
        );
        let get_ident = format_ident!("get_{field_ident}");
        let get_ident_mut = format_ident!("get_{field_ident}_mut");

        aux_struct_defs.push(quote! {
            #[derive(Clone, Copy, Default, PartialEq, Eq, Hash, ::pico_macros::Singleton)]
            pub struct #counter_ident(u64);
            impl ::pico::Counter for #counter_ident {
                #[inline]
                fn increment(self) -> Self { Self(self.0.wrapping_add(1)) }
            }
        });

        struct_impl.push(quote! {
            const #proj_ident: ::pico::Projector<#struct_ident #type_generics, #field_ty> = |db: &#struct_ident #type_generics| &db.#field_ident;
            const #proj_ident_mut: ::pico::ProjectorMut<#struct_ident #type_generics, #field_ty> = |db: &mut #struct_ident #type_generics| &mut db.#field_ident;

            #[inline]
            pub fn #get_ident(&self) -> ::pico::View<'_, #struct_ident #type_generics, #field_ty, #counter_ident> {
                ::pico::View::new(self, Self::#proj_ident)
            }

            #[inline]
            pub fn #get_ident_mut(&mut self) -> ::pico::MutView<'_, #struct_ident #type_generics, #field_ty, #counter_ident> {
                ::pico::MutView::new(self, Self::#proj_ident_mut)
            }
        });
    }

    let output = quote! {
        #(#aux_struct_defs)*

        impl #impl_generics #struct_ident #type_generics #where_clause {
            #(#struct_impl)*
        }

        impl #impl_generics ::pico::Database for #struct_ident #type_generics #where_clause {
            #[inline]
            fn get_storage(&self) -> &::pico::Storage<Self> {
                &self.storage
            }

            #[inline]
            fn get<T: 'static>(&self, id: ::pico::SourceId<T>) -> &T {
                self.storage.get(id)
            }

            #[inline]
            fn get_singleton<T: ::pico::Singleton + 'static>(&self) -> Option<&T> {
                self.storage.get_singleton::<T>()
            }

            #[inline]
            fn intern_value<T: Clone + std::hash::Hash + ::pico::DynEq + 'static>(&self, value: T) -> ::pico::MemoRef<T> {
                ::pico::intern_value(self, value)
            }

            #[inline]
            fn intern_ref<T: Clone + std::hash::Hash + ::pico::DynEq + 'static>(&self, value: &T) -> ::pico::MemoRef<T> {
                ::pico::intern_ref(self, value)
            }

            #[inline]
            fn set<T: ::pico::Source + ::pico::DynEq>(&mut self, source: T) -> ::pico::SourceId<T> {
                self.storage.set(source)
            }

            #[inline]
            fn remove<T>(&mut self, id: ::pico::SourceId<T>) {
                self.storage.remove(id)
            }

            #[inline]
            fn remove_singleton<T: ::pico::Singleton + 'static>(&mut self) {
                self.storage.remove_singleton::<T>()
            }

            #[inline]
            fn run_garbage_collection(&mut self) {
                self.storage.run_garbage_collection()
            }
        }

        impl #impl_generics ::pico::DatabaseDyn for #struct_ident #type_generics #where_clause {
            #[inline]
            fn get_storage_dyn(&self) -> &dyn ::pico::StorageDyn {
                use ::pico::Database;
                self.get_storage()
            }
        }
    };

    output.into()
}

fn has_attr(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.path().is_ident(name))
}
