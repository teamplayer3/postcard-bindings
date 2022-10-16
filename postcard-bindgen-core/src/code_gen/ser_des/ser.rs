use genco::{
    lang::js::Tokens,
    prelude::JavaScript,
    quote, quote_in,
    tokens::{quoted, FormatInto},
};

use crate::{
    code_gen::{utils::semicolon_chain, JS_ENUM_VARIANT_VALUE},
    registry::{BindingType, StructField},
    type_info::{bool_to_js_bool, ArrayMeta, JsType, NumberMeta, ObjectMeta},
    utils::StrExt,
};

#[derive(Debug, Clone, Copy)]
enum FieldAccessor<'a> {
    Object(&'a str),
    Array(usize),
    Direct,
}

impl FormatInto<JavaScript> for FieldAccessor<'_> {
    fn format_into(self, tokens: &mut genco::Tokens<JavaScript>) {
        quote_in! { *tokens =>
            $(match self {
                FieldAccessor::Array(i) => [$i],
                FieldAccessor::Object(n) => .$n,
                FieldAccessor::Direct => ()
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InnerTypeAccess {
    Direct,
    EnumInner,
}

impl FormatInto<JavaScript> for InnerTypeAccess {
    fn format_into(self, tokens: &mut genco::Tokens<JavaScript>) {
        quote_in! { *tokens =>
            $(match self {
                InnerTypeAccess::Direct => (),
                InnerTypeAccess::EnumInner => .$JS_ENUM_VARIANT_VALUE
            })
        }
    }
}

/// Generates a specific accessor based on the type and how the value which is serialized
/// can be access.
///
/// ## Example
/// ```text,ignore
/// ty: Array<u8>                   (type to be serialized)
/// field_access: Direct            (path to value in outer type)
/// field_accessor: Object<test>    (outer type value access)
///
/// -> s.serialize_array((s, v) => s.serialize_number(U8_BYTES, false, v), v.test)
/// ```
fn gen_accessor(
    ty: &JsType,
    field_access: InnerTypeAccess,
    field_accessor: FieldAccessor,
) -> Tokens {
    let accessor_type = ty.as_func_name();
    match ty {
        JsType::Array(a) => gen_accessor_array(accessor_type, a, field_access, field_accessor),
        JsType::Number(n) => gen_accessor_number(accessor_type, n, field_access, field_accessor),
        JsType::String(_) => gen_accessor_simple(accessor_type, field_access, field_accessor),
        JsType::Object(o) => gen_accessor_object(o, field_access, field_accessor),
        JsType::Optional(t) => gen_accessor_optional(t, field_access, field_accessor),
    }
}

fn gen_accessor_optional(
    inner_type: &JsType,
    field_access: InnerTypeAccess,
    field_accessor: FieldAccessor,
) -> Tokens {
    let type_accessor = gen_accessor(inner_type, field_access, field_accessor);
    quote!(if (v$field_access$field_accessor !== undefined) { s.serialize_number(U32_BYTES, false, 1); $type_accessor } else { s.serialize_number(U32_BYTES, false, 0) })
}

// quote!(s.serialize_$(ty.as_func_name())(v.$(field.as_ref())))
// quote!(s.serialize_$(ty.as_func_name())(v[$index]))
// quote!(s.serialize_$(ty.as_func_name())(v.inner$field_access))
fn gen_accessor_simple(
    accessor_type: impl AsRef<str>,
    field_access: InnerTypeAccess,
    field_accessor: FieldAccessor,
) -> Tokens {
    let accessor_type = accessor_type.as_ref();
    quote!(s.serialize_$accessor_type(v$field_access$field_accessor))
}

// quote!(s.serialize_$(ty.as_func_name())($(m.as_byte_js_string()),$(bool_to_js_bool(m.signed)),v.$field))
// quote!(s.serialize_$(ty.as_func_name())($(m.as_byte_js_string()),$(bool_to_js_bool(m.signed)),v[$index]))
// quote!(s.serialize_$(ty.as_func_name())($(m.as_byte_js_string()),$(bool_to_js_bool(m.signed)),v.inner$field_accessor))
fn gen_accessor_number(
    accessor_type: impl AsRef<str>,
    number_meta: &NumberMeta,
    field_access: InnerTypeAccess,
    field_accessor: FieldAccessor,
) -> Tokens {
    let accessor_type = accessor_type.as_ref();
    let byte_amount_str = number_meta.as_byte_js_string();
    let signed = bool_to_js_bool(number_meta.signed);
    quote!(s.serialize_$accessor_type($byte_amount_str,$signed,v$field_access$field_accessor))
}

// quote!(s.serialize_$(ty.as_func_name())((s, v) => $(gen_ser_function_nested(items_type)),v.$field))
// quote!(s.serialize_$(ty.as_func_name())((s, v) => $(gen_ser_function_nested(items_type)),v[$index]))
// quote!(s.serialize_$(ty.as_func_name())((s, v) => $(gen_ser_function_nested(items_type)),v.inner$field_accessor))
fn gen_accessor_array(
    accessor_type: impl AsRef<str>,
    array_meta: &ArrayMeta,
    field_access: InnerTypeAccess,
    field_accessor: FieldAccessor,
) -> Tokens {
    let accessor_type = accessor_type.as_ref();
    let inner_type_accessor = gen_accessor(
        &array_meta.items_type,
        InnerTypeAccess::Direct,
        FieldAccessor::Direct,
    );
    quote!(s.serialize_$accessor_type((s, v) => $inner_type_accessor,v$field_access$field_accessor))
}

// quote!(serialize_$(obj_meta.name.to_obj_identifier())(s, v.$(field.as_ref())))
// quote!(serialize_$(obj_meta.name.to_case(Case::Snake).to_uppercase())(s, v[$index]))
// quote!(serialize_$(obj_meta.name.to_obj_identifier())(s, v.inner$field_access))
fn gen_accessor_object(
    obj_meta: &ObjectMeta,
    field_access: InnerTypeAccess,
    field_accessor: FieldAccessor,
) -> Tokens {
    let obj_ident = obj_meta.name.to_obj_identifier();
    quote!(serialize_$obj_ident(s, v$field_access$field_accessor))
}

fn gen_accessors_tuple(fields: impl AsRef<[JsType]>, field_access: InnerTypeAccess) -> Tokens {
    semicolon_chain(
        fields
            .as_ref()
            .iter()
            .enumerate()
            .map(|(index, field)| gen_accessor(field, field_access, FieldAccessor::Array(index))),
    )
}

fn gen_accessors_struct(
    fields: impl AsRef<[StructField]>,
    field_access: InnerTypeAccess,
) -> Tokens {
    semicolon_chain(fields.as_ref().iter().map(|field| {
        gen_accessor(
            &field.js_type,
            field_access,
            FieldAccessor::Object(field.name),
        )
    }))
}

pub fn gen_serialize_func(defines: impl AsRef<[BindingType]>) -> Tokens {
    let switch_body = gen_ser_cases(defines);
    quote!(
        module.exports.serialize = (type, value) => {
            if (!(typeof type === "string")) {
                throw "type must be a string"
            }
            const s = new Serializer()
            switch (type) {
                $switch_body
            }
            return s.finish()
        }
    )
}

fn gen_ser_cases(defines: impl AsRef<[BindingType]>) -> Tokens {
    semicolon_chain(defines.as_ref().iter().map(gen_ser_case))
}

fn gen_ser_case(define: &BindingType) -> Tokens {
    let name = define.inner_name();
    let case_str = quoted(name);
    let type_name = name.to_obj_identifier();
    quote!(case $case_str: if (is_$(type_name.as_str())(value)) { serialize_$(type_name)(s, value) } else throw "value has wrong format"; break)
}

pub mod strukt {
    use genco::{lang::js::Tokens, quote};

    use crate::{registry::StructField, utils::StrExt};

    use super::{gen_accessors_struct, InnerTypeAccess};

    pub fn gen_function(obj_name: impl AsRef<str>, fields: impl AsRef<[StructField]>) -> Tokens {
        let obj_name_upper = obj_name.as_ref().to_obj_identifier();
        let body = gen_accessors_struct(fields, InnerTypeAccess::Direct);
        quote! {
            const serialize_$(obj_name_upper) = (s, v) => { $body }
        }
    }
}

pub mod tuple_struct {
    use genco::{lang::js::Tokens, quote};

    use crate::{type_info::JsType, utils::StrExt};

    use super::{gen_accessors_tuple, InnerTypeAccess};

    pub fn gen_function(obj_name: impl AsRef<str>, fields: impl AsRef<[JsType]>) -> Tokens {
        let obj_name_upper = obj_name.as_ref().to_obj_identifier();
        let body = gen_accessors_tuple(fields, InnerTypeAccess::Direct);
        quote! {
            const serialize_$(obj_name_upper) = (s, v) => { $body }
        }
    }
}

pub mod enum_ty {
    use genco::{
        lang::js::Tokens,
        prelude::JavaScript,
        quote, quote_in,
        tokens::{quoted, FormatInto},
    };

    use crate::{
        code_gen::{utils::semicolon_chain, JS_ENUM_VARIANT_KEY},
        registry::{EnumVariant, EnumVariantType},
        utils::StrExt,
    };

    use super::{gen_accessors_struct, gen_accessors_tuple, InnerTypeAccess};

    pub fn gen_function(obj_name: impl AsRef<str>, variants: impl AsRef<[EnumVariant]>) -> Tokens {
        let obj_name_upper = obj_name.as_ref().to_obj_identifier();
        let enumerated_variants = variants.as_ref().iter().enumerate();
        let switch_body = semicolon_chain(
            enumerated_variants.map(|(index, variant)| gen_case_for_variant(index, variant)),
        );
        quote! {
            const serialize_$(obj_name_upper) = (s, v) => {
                switch (v.$JS_ENUM_VARIANT_KEY) {
                    $switch_body
                }
            }
        }
    }

    enum CaseBody {
        Body(Tokens),
        None,
    }

    impl FormatInto<JavaScript> for CaseBody {
        fn format_into(self, tokens: &mut genco::Tokens<JavaScript>) {
            quote_in! { *tokens =>
                $(match self {
                    CaseBody::Body(b) => $b;,
                    CaseBody::None => ()
                })
            }
        }
    }

    fn gen_case_for_variant(index: usize, variant: &EnumVariant) -> Tokens {
        let variant_name = quoted(variant.name);
        let body = match &variant.inner_type {
            EnumVariantType::Empty => CaseBody::None,
            EnumVariantType::Tuple(fields) => {
                CaseBody::Body(gen_accessors_tuple(fields, InnerTypeAccess::EnumInner))
            }
            EnumVariantType::NewType(fields) => {
                CaseBody::Body(gen_accessors_struct(fields, InnerTypeAccess::EnumInner))
            }
        };

        quote!(case $variant_name: s.serialize_number(U32_BYTES, false, $index); $body break)
    }
}
