use alloc::vec::Vec;

use crate::{
    type_info::{GenJsBinding, ValueType},
    utils::ContainerPath,
};

#[derive(Debug, Clone)]
pub struct Container {
    pub path: ContainerPath<'static>,
    pub name: &'static str,
    pub r#type: BindingType,
}

#[derive(Debug, Clone)]
pub enum BindingType {
    Struct(StructType),
    TupleStruct(TupleStructType),
    UnitStruct(UnitStructType),
    Enum(EnumType),
}

#[derive(Debug, Clone)]
// encoded into | variant index | (inner)
pub struct EnumType {
    pub variants: Vec<EnumVariant>,
}

impl EnumType {
    pub fn new() -> Self {
        Self {
            variants: Default::default(),
        }
    }

    // index is set based on order of variant registration
    pub fn register_variant(&mut self, name: &'static str) {
        self.variants.push(EnumVariant {
            index: self.variants.len(),
            name,
            inner_type: EnumVariantType::Empty,
        });
    }

    pub fn register_variant_tuple(&mut self, name: &'static str, fields: TupleFields) {
        self.variants.push(EnumVariant {
            index: self.variants.len(),
            name,
            inner_type: EnumVariantType::Tuple(fields.into_inner()),
        });
    }

    pub fn register_unnamed_struct(&mut self, name: &'static str, fields: StructFields) {
        self.variants.push(EnumVariant {
            index: self.variants.len(),
            name,
            inner_type: EnumVariantType::NewType(fields.into_inner()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub index: usize,
    pub name: &'static str,
    pub inner_type: EnumVariantType,
}

impl AsRef<EnumVariant> for EnumVariant {
    fn as_ref(&self) -> &EnumVariant {
        self
    }
}

#[derive(Debug, Clone)]
pub enum EnumVariantType {
    Empty,
    Tuple(Vec<ValueType>),
    // for unnamed structs create struct with custom name ( __EnumName_Struct1)
    NewType(Vec<StructField>),
}

#[derive(Debug, Clone)]
pub struct StructType {
    pub fields: Vec<StructField>,
}

impl StructType {
    pub fn new() -> Self {
        Self {
            fields: Default::default(),
        }
    }

    pub fn register_field<T: GenJsBinding>(&mut self, name: &'static str) {
        self.fields.push(StructField {
            name,
            v_type: T::get_type(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TupleStructType {
    pub fields: Vec<ValueType>,
}

impl TupleStructType {
    pub fn new() -> Self {
        Self {
            fields: Default::default(),
        }
    }

    pub fn register_field<T: GenJsBinding>(&mut self) {
        self.fields.push(T::get_type())
    }
}

#[derive(Debug, Clone)]
pub struct UnitStructType;

impl UnitStructType {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: &'static str,
    pub v_type: ValueType,
}

#[derive(Debug, Default)]
pub struct StructFields(Vec<StructField>);

impl StructFields {
    pub fn register_field<T: GenJsBinding>(&mut self, name: &'static str) {
        self.0.push(StructField {
            name,
            v_type: T::get_type(),
        })
    }

    fn into_inner(self) -> Vec<StructField> {
        self.0
    }
}

#[derive(Default)]
pub struct TupleFields(Vec<ValueType>);

impl TupleFields {
    pub fn register_field<T: GenJsBinding>(&mut self) {
        self.0.push(T::get_type())
    }

    fn into_inner(self) -> Vec<ValueType> {
        self.0
    }
}

#[derive(Debug, Default)]
pub struct BindingsRegistry(Vec<Container>);

impl BindingsRegistry {
    pub fn register_struct_binding(
        &mut self,
        name: &'static str,
        path: ContainerPath<'static>,
        value: StructType,
    ) {
        self.0.push(Container {
            path,
            name,
            r#type: BindingType::Struct(value),
        });
    }

    pub fn register_tuple_struct_binding(
        &mut self,
        name: &'static str,
        path: ContainerPath<'static>,
        value: TupleStructType,
    ) {
        self.0.push(Container {
            path,
            name,
            r#type: BindingType::TupleStruct(value),
        });
    }

    pub fn register_unit_struct_binding(
        &mut self,
        name: &'static str,
        path: ContainerPath<'static>,
        value: UnitStructType,
    ) {
        self.0.push(Container {
            path,
            name,
            r#type: BindingType::UnitStruct(value),
        });
    }

    pub fn register_enum_binding(
        &mut self,
        name: &'static str,
        path: ContainerPath<'static>,
        value: EnumType,
    ) {
        self.0.push(Container {
            path,
            name,
            r#type: BindingType::Enum(value),
        });
    }

    pub fn into_entries(self) -> Vec<Container> {
        self.0
    }
}

pub trait JsBindings {
    fn create_bindings(registry: &mut BindingsRegistry);
}

#[cfg(test)]
mod test {
    use crate::registry::{
        BindingsRegistry, EnumType, JsBindings, StructFields, StructType, TupleFields,
        TupleStructType,
    };

    #[test]
    fn test_registry_struct() {
        #[allow(unused)]
        struct Test {
            a: u8,
            b: u16,
            c: &'static str,
        }

        impl JsBindings for Test {
            fn create_bindings(registry: &mut BindingsRegistry) {
                let mut ty = StructType::new();

                ty.register_field::<u8>("a".into());
                ty.register_field::<u16>("b".into());
                ty.register_field::<&str>("c".into());

                registry.register_struct_binding("Test", "".into(), ty);
            }
        }

        let mut registry = BindingsRegistry::default();
        Test::create_bindings(&mut registry);
    }

    #[test]
    fn test_registry_tuple_struct() {
        #[allow(dead_code)]
        struct Test(u8, &'static str, &'static [u8]);

        impl JsBindings for Test {
            fn create_bindings(registry: &mut BindingsRegistry) {
                let mut ty = TupleStructType::new();

                ty.register_field::<u8>();
                ty.register_field::<&str>();
                ty.register_field::<&[u8]>();

                registry.register_tuple_struct_binding("Test", "".into(), ty);
            }
        }

        let mut registry = BindingsRegistry::default();
        Test::create_bindings(&mut registry);
    }

    #[test]
    fn test_registry_enum() {
        #[allow(unused)]
        enum Test {
            A,
            B(u8),
            C { a: &'static str, b: u16 },
        }

        impl JsBindings for Test {
            fn create_bindings(registry: &mut BindingsRegistry) {
                let mut ty = EnumType::new();

                ty.register_variant("A".into());

                let mut fields = TupleFields::default();
                fields.register_field::<u8>();
                ty.register_variant_tuple("B".into(), fields);

                let mut fields = StructFields::default();
                fields.register_field::<&str>("a".into());
                fields.register_field::<u16>("b".into());
                ty.register_unnamed_struct("C".into(), fields);

                registry.register_enum_binding("Test", "".into(), ty);
            }
        }

        let mut registry = BindingsRegistry::default();
        Test::create_bindings(&mut registry);
    }
}
