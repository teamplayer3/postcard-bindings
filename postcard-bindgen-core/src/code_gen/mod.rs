use genco::{prelude::js::Tokens, quote};

use crate::registry::BindingType;

use self::{
    ser_des::{
        gen_deserialize_func, gen_ser_des_classes, gen_ser_des_functions, gen_serialize_func,
        CodeConfig,
    },
    type_checking::gen_type_checkings,
};

mod generateable;
pub mod ser_des;
pub mod type_checking;
mod utils;

const JS_ENUM_VARIANT_KEY: &str = "tag";
const JS_ENUM_VARIANT_VALUE: &str = "value";
const JS_OBJECT_VARIABLE: &str = "v";

pub fn generate_js(tys: impl AsRef<[BindingType]>, js_type_checks: bool) -> Tokens {
    let ser_des_body = gen_ser_des_functions(&tys);
    let ser_des_class_config = CodeConfig {
        incl_bounds_checking: js_type_checks,
    };
    quote!(
        $(gen_ser_des_classes(ser_des_class_config))
        $ser_des_body
        $(if js_type_checks => $(gen_type_checkings(&tys)))
        $(gen_serialize_func(&tys, js_type_checks))
        $(gen_deserialize_func(tys))
    )
}
