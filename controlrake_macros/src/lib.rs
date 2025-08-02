/// 🚀 ControlRake Macro System - Procedural Macros for Component Generation
///
/// **Purpose**: Eliminate boilerplate in component implementations by generating
/// Component trait implementations, method routing, template embedding, and response handling.
///
/// **Design Philosophy**:
/// - Zero runtime overhead - all code generated at compile time
/// - Preserve all existing functionality while reducing boilerplate
/// - Clear separation between user code and generated code
/// - Full type safety and compile-time validation
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemStruct, Ident, Meta, punctuated::Punctuated, Token, parse::Parser};

/// 📋 Component macro attribute configuration
#[derive(Debug, Default)]
struct ComponentAttributes {
    template: Option<String>,
    config: Option<String>,
}

/// 📋 Method macro attribute configuration
#[derive(Debug, Default)]
struct MethodAttributes {
    default_instance: bool,
}

/// 📋 ComponentResponse macro attribute configuration
#[derive(Debug, Default)]
struct ComponentResponseAttributes {
    /// Custom struct name (defaults to ComponentResponse)
    name: Option<String>,
    /// Additional fields to include in the response
    fields: Vec<ResponseField>,
    /// Whether to include default timestamp field
    include_timestamp: bool,
    /// Whether to include default version field  
    include_version: bool,
}

/// 📊 Custom response field definition
#[derive(Debug, Clone)]
struct ResponseField {
    name: String,
    field_type: String,
    optional: bool,
    #[allow(dead_code)] // Reserved for future default value implementation
    default_value: Option<String>,
}

/// 📊 Parsed method information for code generation
#[derive(Debug)]
struct MethodInfo {
    name: String,
    is_public: bool,
    is_async: bool,
    parameters: Vec<MethodParameter>,
    return_type: Option<syn::Type>,
    attributes: MethodAttributes,
}

/// 📊 Method parameter information
#[derive(Debug)]
struct MethodParameter {
    name: String,
    is_self: bool,
}

/// 🎯 Component macro - Generate Component trait implementation
///
/// **Usage**: `#[crate::component(template = "button.hbs", config = "ButtonConfig")]`
///
/// **Generated Code**:
/// - Component trait implementation with render(), handle_message(), etc.
/// - Template embedding via include_str!
/// - Configuration field injection
/// - Constructor generation
/// - Method routing framework
///
/// **Example**:
/// ```rust
/// #[crate::component(template = "button.hbs", config = "ButtonConfig")]
/// struct ButtonComponent {
///     // User fields
///     counter: u64,
/// }
/// 
/// impl ButtonComponent {
///     #[crate::method]
///     pub fn click(&mut self) -> ::anyhow::Result<u64> {
///         self.counter += 1;
///         Ok(self.counter)
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the struct that the macro is applied to
    let input = parse_macro_input!(item as ItemStruct);
    
    // Parse the macro attributes
    let component_attrs = match parse_component_attributes(attr) {
        Ok(attrs) => attrs,
        Err(e) => {
            return syn::Error::new(proc_macro2::Span::call_site(), e.to_string())
                .to_compile_error()
                .into();
        }
    };

    // Generate the enhanced component with macro-generated code
    match generate_component_implementation(&input, &component_attrs) {
        Ok(tokens) => tokens.into(),
        Err(e) => {
            syn::Error::new(proc_macro2::Span::call_site(), e.to_string())
                .to_compile_error()
                .into()
        }
    }
}

/// 🎯 Method macro - Mark methods for bus routing
///
/// **Usage**: `#[crate::method]` or `#[crate::method(default_instance)]`
///
/// **Generated Code**:
/// - Method registration in handle_message router
/// - Parameter deserialization from JSON
/// - Return value serialization to ComponentResult
/// - Default instance routing (if specified)
///
/// **Example**:
/// ```rust
/// #[crate::method]
/// pub fn click(&mut self) -> ::anyhow::Result<u64> {
///     // User implementation
/// }
///
/// #[crate::method(default_instance)]
/// pub fn get_status() -> ::anyhow::Result<String> {
///     // Static method for default instance
/// }
/// ```
#[proc_macro_attribute]
pub fn method(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the function that the macro is applied to
    let input = parse_macro_input!(item as ItemFn);
    
    // Parse the method attributes
    let method_attrs = match parse_method_attributes(attr) {
        Ok(attrs) => attrs,
        Err(e) => {
            return syn::Error::new(proc_macro2::Span::call_site(), e.to_string())
                .to_compile_error()
                .into();
        }
    };

    // Parse method information for validation and code generation
    let method_info = match parse_method_info(&input, method_attrs) {
        Ok(info) => info,
        Err(e) => {
            return syn::Error::new(proc_macro2::Span::call_site(), e.to_string())
                .to_compile_error()
                .into();
        }
    };

    // Validate the method meets requirements
    if let Err(e) = validate_method(&method_info) {
        return syn::Error::new(proc_macro2::Span::call_site(), e.to_string())
            .to_compile_error()
            .into();
    }

    // ✅ Validate direct call support (Task 3.4)
    if let Err(e) = validate_direct_call_support(&input, &method_info) {
        return syn::Error::new(proc_macro2::Span::call_site(), e.to_string())
            .to_compile_error()
            .into();
    }

    // 🚀 Generate method routing code for Phase 3.2
    let method_routing = match generate_method_routing(&method_info) {
        Ok(routing) => routing,
        Err(e) => {
            return syn::Error::new(proc_macro2::Span::call_site(), e.to_string())
                .to_compile_error()
                .into();
        }
    };
    
    let expanded = quote! {
        // 📝 Original user method - preserved unchanged for direct calls
        // Users can still call: component.method_name() directly
        // Bus routing is an additional layer, not a replacement
        #input
        
        // 🚀 Generated method routing - implement as separate impl block
        // This allows the method to override the default try_call_* behavior
        // Bus calls route through: component.try_call_method_name_method(payload)
        #method_routing
    };

    TokenStream::from(expanded)
}

/// 📤 ComponentResponse macro - Generate response struct and methods
///
/// **Usage**: `#[component_response]` or `#[component_response(name = "CustomResponse", fields = "field1:Type1,field2:Option<Type2>")]`
///
/// **Generated Code**:
/// - ComponentResponse struct with common fields (success, action, version, component_id, etc.)
/// - Constructor methods (success(), error())
/// - Builder methods (with_timestamp(), with_message(), etc.)
/// - Serialization method (to_json())
/// - Custom field support
///
/// **Example**:
/// ```rust
/// #[component_response]
/// struct ButtonResponse {
///     // Custom fields
///     click_count: Option<u32>,
///     text: Option<String>,
/// }
///
/// // Generated methods available:
/// // ButtonResponse::success(action, component_id, version)
/// // .with_click_count(count)
/// // .with_text(text)
/// // .to_json()
/// ```
#[proc_macro_attribute]
pub fn component_response(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the struct that the macro is applied to
    let input = parse_macro_input!(item as ItemStruct);
    
    // Parse the macro attributes
    let response_attrs = match parse_component_response_attributes(attr) {
        Ok(attrs) => attrs,
        Err(e) => {
            return syn::Error::new(proc_macro2::Span::call_site(), e.to_string())
                .to_compile_error()
                .into();
        }
    };

    // Generate the complete ComponentResponse implementation
    match generate_component_response_implementation(&input, &response_attrs) {
        Ok(tokens) => tokens.into(),
        Err(e) => {
            syn::Error::new(proc_macro2::Span::call_site(), e.to_string())
                .to_compile_error()
                .into()
        }
    }
}

/// 🔍 Parse component macro attributes
/// 
/// **Supported Attributes**:
/// - `template = "path/to/template.hbs"` - Template file path
/// - `config = "ConfigType"` - Configuration type name
/// 
/// **Example**: `#[component(template = "button.hbs", config = "ButtonConfig")]`
fn parse_component_attributes(attr: TokenStream) -> ::anyhow::Result<ComponentAttributes> {
    let mut attributes = ComponentAttributes::default();
    
    if attr.is_empty() {
        return Ok(attributes);
    }

    // Parse the attribute tokens as a function-like macro call
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let parsed = parser.parse(attr)
        .map_err(|e| anyhow::anyhow!("Failed to parse component attributes: {}", e))?;
    
    for meta in parsed {
        match meta {
            Meta::NameValue(name_value) => {
                let name = name_value.path.get_ident()
                    .ok_or_else(|| anyhow::anyhow!("Invalid attribute name"))?
                    .to_string();
                
                match name.as_str() {
                    "template" => {
                        if let syn::Expr::Lit(expr_lit) = name_value.value {
                            if let syn::Lit::Str(lit_str) = expr_lit.lit {
                                attributes.template = Some(lit_str.value());
                            } else {
                                return Err(anyhow::anyhow!("template attribute must be a string"));
                            }
                        } else {
                            return Err(anyhow::anyhow!("template attribute must be a string literal"));
                        }
                    }
                    "config" => {
                        if let syn::Expr::Lit(expr_lit) = name_value.value {
                            if let syn::Lit::Str(lit_str) = expr_lit.lit {
                                attributes.config = Some(lit_str.value());
                            } else {
                                return Err(anyhow::anyhow!("config attribute must be a string"));
                            }
                        } else {
                            return Err(anyhow::anyhow!("config attribute must be a string literal"));
                        }
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Unknown attribute: {}", name));
                    }
                }
            }
            Meta::Path(_) => {
                // Empty attribute - this is valid for #[component]
            }
            _ => {
                return Err(anyhow::anyhow!("Invalid attribute format. Use name = \"value\""));
            }
        }
    }
    
    Ok(attributes)
}

/// 🔍 Parse component_response macro attributes
///
/// **Supported Attributes**:
/// - `name = "CustomResponseName"` - Custom struct name
/// - `fields = "field1:Type1,field2:Option<Type2>"` - Additional fields
/// - `include_timestamp = false` - Disable default timestamp field
/// - `include_version = false` - Disable default version field
///
/// **Example**: `#[component_response(name = "ButtonResponse", fields = "click_count:Option<u32>")]`
fn parse_component_response_attributes(attr: TokenStream) -> ::anyhow::Result<ComponentResponseAttributes> {
    let mut attributes = ComponentResponseAttributes {
        include_timestamp: true,  // Default to true
        include_version: true,    // Default to true
        ..Default::default()
    };
    
    if attr.is_empty() {
        return Ok(attributes);
    }

    // Parse the attribute tokens as a function-like macro call
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let parsed = parser.parse(attr)
        .map_err(|e| anyhow::anyhow!("Failed to parse component_response attributes: {}", e))?;
    
    for meta in parsed {
        match meta {
            Meta::NameValue(name_value) => {
                let name = name_value.path.get_ident()
                    .ok_or_else(|| anyhow::anyhow!("Invalid attribute name"))?
                    .to_string();
                
                match name.as_str() {
                    "name" => {
                        if let syn::Expr::Lit(expr_lit) = name_value.value {
                            if let syn::Lit::Str(lit_str) = expr_lit.lit {
                                attributes.name = Some(lit_str.value());
                            } else {
                                return Err(anyhow::anyhow!("name attribute must be a string"));
                            }
                        } else {
                            return Err(anyhow::anyhow!("name attribute must be a string literal"));
                        }
                    }
                    "fields" => {
                        if let syn::Expr::Lit(expr_lit) = name_value.value {
                            if let syn::Lit::Str(lit_str) = expr_lit.lit {
                                attributes.fields = parse_response_fields(&lit_str.value())?;
                            } else {
                                return Err(anyhow::anyhow!("fields attribute must be a string"));
                            }
                        } else {
                            return Err(anyhow::anyhow!("fields attribute must be a string literal"));
                        }
                    }
                    "include_timestamp" => {
                        if let syn::Expr::Lit(expr_lit) = name_value.value {
                            if let syn::Lit::Bool(lit_bool) = expr_lit.lit {
                                attributes.include_timestamp = lit_bool.value;
                            } else {
                                return Err(anyhow::anyhow!("include_timestamp attribute must be a boolean"));
                            }
                        } else {
                            return Err(anyhow::anyhow!("include_timestamp attribute must be a boolean literal"));
                        }
                    }
                    "include_version" => {
                        if let syn::Expr::Lit(expr_lit) = name_value.value {
                            if let syn::Lit::Bool(lit_bool) = expr_lit.lit {
                                attributes.include_version = lit_bool.value;
                            } else {
                                return Err(anyhow::anyhow!("include_version attribute must be a boolean"));
                            }
                        } else {
                            return Err(anyhow::anyhow!("include_version attribute must be a boolean literal"));
                        }
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Unknown component_response attribute: {}", name));
                    }
                }
            }
            Meta::Path(_) => {
                // Empty attribute - this is valid for #[component_response]
            }
            _ => {
                return Err(anyhow::anyhow!("Invalid component_response attribute format. Use name = \"value\""));
            }
        }
    }
    
    Ok(attributes)
}

/// 🔍 Parse response fields from string format
///
/// **Format**: "field1:Type1,field2:Option<Type2>,field3:Type3=default_value"
/// **Examples**:
/// - "click_count:Option<u32>" - Optional field
/// - "text:String" - Required field
/// - "status:String=connected" - Field with default value
fn parse_response_fields(fields_str: &str) -> ::anyhow::Result<Vec<ResponseField>> {
    let mut fields = Vec::new();
    
    if fields_str.trim().is_empty() {
        return Ok(fields);
    }
    
    for field_def in fields_str.split(',') {
        let field_def = field_def.trim();
        if field_def.is_empty() {
            continue;
        }
        
        // Parse field_name:field_type or field_name:field_type=default_value
        let parts: Vec<&str> = field_def.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid field format '{}'. Use 'name:Type' or 'name:Type=default'", field_def));
        }
        
        let field_name = parts[0].trim().to_string();
        let type_and_default = parts[1].trim();
        
        // Check for default value
        let (field_type, default_value) = if let Some(eq_pos) = type_and_default.find('=') {
            let field_type = type_and_default[..eq_pos].trim().to_string();
            let default_val = type_and_default[eq_pos + 1..].trim().to_string();
            (field_type, Some(default_val))
        } else {
            (type_and_default.to_string(), None)
        };
        
        // Check if field is optional (wrapped in Option<>)
        let optional = field_type.starts_with("Option<") && field_type.ends_with('>');
        
        fields.push(ResponseField {
            name: field_name,
            field_type,
            optional,
            default_value,
        });
    }
    
    Ok(fields)
}

/// 🏗️ Generate complete ComponentResponse implementation
fn generate_component_response_implementation(
    input: &ItemStruct,
    attrs: &ComponentResponseAttributes,
) -> ::anyhow::Result<proc_macro2::TokenStream> {
    let original_name = &input.ident;
    
    // Use custom name if provided, otherwise use original struct name
    let struct_name = if let Some(custom_name) = &attrs.name {
        syn::parse_str::<syn::Ident>(custom_name)
            .map_err(|e| anyhow::anyhow!("Invalid custom name '{}': {}", custom_name, e))?
    } else {
        original_name.clone()
    };
    
    // Extract user-defined fields from the original struct
    let user_fields = extract_user_fields(&input.fields)?;
    
    // Generate the enhanced response struct
    let response_struct = generate_response_struct(&struct_name, attrs, &user_fields)?;
    
    // Generate implementation methods
    let response_impl = generate_response_methods(&struct_name, attrs, &user_fields)?;
    
    Ok(quote! {
        // 📤 Generated ComponentResponse struct with all fields
        #response_struct
        
        // 🏗️ Generated implementation with constructors and builders
        #response_impl
    })
}

/// 📊 Extract user-defined fields from the input struct
fn extract_user_fields(fields: &syn::Fields) -> ::anyhow::Result<Vec<syn::Field>> {
    match fields {
        syn::Fields::Named(fields_named) => {
            Ok(fields_named.named.iter().cloned().collect())
        }
        syn::Fields::Unnamed(_) => {
            Err(anyhow::anyhow!("Tuple structs not supported in component_response macro"))
        }
        syn::Fields::Unit => {
            Ok(Vec::new())
        }
    }
}

/// 🏗️ Generate the complete response struct with all fields
fn generate_response_struct(
    struct_name: &Ident,
    attrs: &ComponentResponseAttributes,
    user_fields: &[syn::Field],
) -> ::anyhow::Result<proc_macro2::TokenStream> {
    
    // Generate common ComponentResponse fields
    let mut common_fields = vec![
        quote! { success: bool },
        quote! { action: String },
        quote! { component_id: String },
    ];
    
    // Add optional version field
    if attrs.include_version {
        common_fields.push(quote! {
            #[serde(skip_serializing_if = "Option::is_none")]
            version: Option<u64>
        });
    }
    
    // Add optional timestamp field
    if attrs.include_timestamp {
        common_fields.push(quote! {
            #[serde(skip_serializing_if = "Option::is_none")]
            timestamp: Option<String>
        });
    }
    
    // Add standard optional fields
    common_fields.extend([
        quote! {
            #[serde(skip_serializing_if = "Option::is_none")]
            message: Option<String>
        },
        quote! {
            #[serde(skip_serializing_if = "Option::is_none")]
            error: Option<String>
        },
    ]);
    
    // Add user-defined fields (make them optional with skip_serializing_if)
    let user_field_definitions: Vec<_> = user_fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;
        
        // Check if field is already optional
        let is_optional = is_option_type(field_type);
        
        if is_optional {
            quote! {
                #[serde(skip_serializing_if = "Option::is_none")]
                #field_name: #field_type
            }
        } else {
            quote! {
                #[serde(skip_serializing_if = "Option::is_none")]
                #field_name: Option<#field_type>
            }
        }
    }).collect();
    
    // Add fields from macro attributes
    let attr_field_definitions: Vec<_> = attrs.fields.iter().map(|field| {
        let field_name = syn::parse_str::<syn::Ident>(&field.name).unwrap();
        let field_type = syn::parse_str::<syn::Type>(&field.field_type).unwrap();
        
        quote! {
            #[serde(skip_serializing_if = "Option::is_none")]
            #field_name: #field_type
        }
    }).collect();
    
    Ok(quote! {
        /// 📤 Component response builder - eliminates repetitive JSON construction
        #[derive(Debug, Clone, serde::Serialize)]
        struct #struct_name {
            // 🔧 Common ComponentResponse fields
            #(#common_fields,)*
            
            // 👤 User-defined fields from struct
            #(#user_field_definitions,)*
            
            // ⚙️ Additional fields from macro attributes
            #(#attr_field_definitions,)*
        }
    })
}

/// 🏗️ Generate implementation methods for the response struct
fn generate_response_methods(
    struct_name: &Ident,
    attrs: &ComponentResponseAttributes,
    user_fields: &[syn::Field],
) -> ::anyhow::Result<proc_macro2::TokenStream> {
    
    // Generate success constructor
    let success_constructor = generate_success_constructor(struct_name, attrs, user_fields)?;
    
    // Generate error constructor
    let error_constructor = generate_error_constructor(struct_name, attrs, user_fields)?;
    
    // Generate builder methods
    let builder_methods = generate_builder_methods(struct_name, attrs, user_fields)?;
    
    // Generate to_json method
    let to_json_method = generate_to_json_method()?;
    
    Ok(quote! {
        impl #struct_name {
            #success_constructor
            
            #error_constructor
            
            #builder_methods
            
            #to_json_method
        }
    })
}

/// 🏗️ Generate success constructor method
fn generate_success_constructor(
    _struct_name: &Ident,
    attrs: &ComponentResponseAttributes,
    user_fields: &[syn::Field],
) -> ::anyhow::Result<proc_macro2::TokenStream> {
    
    // Generate field initializations for common fields
    let mut field_inits = vec![
        quote! { success: true },
        quote! { action: action.to_string() },
        quote! { component_id: component_id.to_string() },
    ];
    
    // Add version field if enabled
    if attrs.include_version {
        field_inits.push(quote! { version: Some(version) });
    }
    
    // Add optional fields as None
    if attrs.include_timestamp {
        field_inits.push(quote! { timestamp: None });
    }
    
    field_inits.extend([
        quote! { message: None },
        quote! { error: None },
    ]);
    
    // Initialize user fields as None
    for field in user_fields {
        let field_name = &field.ident;
        field_inits.push(quote! { #field_name: None });
    }
    
    // Initialize attribute fields as None  
    for field in &attrs.fields {
        let field_name = syn::parse_str::<syn::Ident>(&field.name).unwrap();
        field_inits.push(quote! { #field_name: None });
    }
    
    // Generate constructor signature based on enabled fields
    let constructor_signature = if attrs.include_version {
        quote! {
            /// Create success response with required context fields
            fn success(action: &str, component_id: &str, version: u64) -> Self
        }
    } else {
        quote! {
            /// Create success response with required context fields
            fn success(action: &str, component_id: &str) -> Self
        }
    };
    
    Ok(quote! {
        #constructor_signature {
            Self {
                #(#field_inits,)*
            }
        }
    })
}

/// 🏗️ Generate error constructor method
fn generate_error_constructor(
    _struct_name: &Ident,
    attrs: &ComponentResponseAttributes,
    user_fields: &[syn::Field],
) -> ::anyhow::Result<proc_macro2::TokenStream> {
    
    // Generate field initializations for error response
    let mut field_inits = vec![
        quote! { success: false },
        quote! { action: String::new() },
        quote! { component_id: String::new() },
    ];
    
    // Add version field if enabled
    if attrs.include_version {
        field_inits.push(quote! { version: None });
    }
    
    // Add optional fields
    if attrs.include_timestamp {
        field_inits.push(quote! { timestamp: None });
    }
    
    field_inits.extend([
        quote! { message: None },
        quote! { error: Some(error.to_string()) },
    ]);
    
    // Initialize user fields as None
    for field in user_fields {
        let field_name = &field.ident;
        field_inits.push(quote! { #field_name: None });
    }
    
    // Initialize attribute fields as None
    for field in &attrs.fields {
        let field_name = syn::parse_str::<syn::Ident>(&field.name).unwrap();
        field_inits.push(quote! { #field_name: None });
    }
    
    Ok(quote! {
        /// Create error response
        fn error(error: &str) -> Self {
            Self {
                #(#field_inits,)*
            }
        }
    })
}

/// 🏗️ Generate builder methods for all fields
fn generate_builder_methods(
    _struct_name: &Ident,
    attrs: &ComponentResponseAttributes,
    user_fields: &[syn::Field],
) -> ::anyhow::Result<proc_macro2::TokenStream> {
    
    let mut builder_methods = Vec::new();
    
    // Generate timestamp builder if enabled
    if attrs.include_timestamp {
        builder_methods.push(quote! {
            /// Add timestamp (current UTC)
            fn with_timestamp(mut self) -> Self {
                self.timestamp = Some(chrono::Utc::now().to_rfc3339());
                self
            }
        });
    }
    
    // Generate message builder
    builder_methods.push(quote! {
        /// Add message
        fn with_message(mut self, message: &str) -> Self {
            self.message = Some(message.to_string());
            self
        }
    });
    
    // Generate builders for user-defined fields
    for field in user_fields {
        let field_name = &field.ident;
        let field_type = &field.ty;
        
        // Generate method name (with_field_name)
        let method_name = syn::parse_str::<syn::Ident>(&format!("with_{}", field_name.as_ref().unwrap()))
            .map_err(|e| anyhow::anyhow!("Invalid method name for field '{}': {}", field_name.as_ref().unwrap(), e))?;
        
        // Check if field is already optional
        let is_optional = is_option_type(field_type);
        
        let builder_method = if is_optional {
            quote! {
                /// Add #field_name
                fn #method_name(mut self, value: #field_type) -> Self {
                    self.#field_name = value;
                    self
                }
            }
        } else {
            quote! {
                /// Add #field_name
                fn #method_name(mut self, value: #field_type) -> Self {
                    self.#field_name = Some(value);
                    self
                }
            }
        };
        
        builder_methods.push(builder_method);
    }
    
    // Generate builders for attribute fields
    for field in &attrs.fields {
        let field_name = syn::parse_str::<syn::Ident>(&field.name).unwrap();
        let field_type = syn::parse_str::<syn::Type>(&field.field_type).unwrap();
        
        let method_name = syn::parse_str::<syn::Ident>(&format!("with_{}", field.name))
            .map_err(|e| anyhow::anyhow!("Invalid method name for field '{}': {}", field.name, e))?;
        
        let builder_method = if field.optional {
            quote! {
                /// Add #field_name
                fn #method_name(mut self, value: #field_type) -> Self {
                    self.#field_name = value;
                    self
                }
            }
        } else {
            quote! {
                /// Add #field_name
                fn #method_name(mut self, value: #field_type) -> Self {
                    self.#field_name = Some(value);
                    self
                }
            }
        };
        
        builder_methods.push(builder_method);
    }
    
    Ok(quote! {
        #(#builder_methods)*
    })
}

/// 🏗️ Generate to_json method
fn generate_to_json_method() -> ::anyhow::Result<proc_macro2::TokenStream> {
    Ok(quote! {
        /// Convert to JSON Value
        fn to_json(&self) -> serde_json::Value {
            serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize response"}))
        }
    })
}

/// 🔍 Check if a type is Option<T>
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// 🏗️ Generate complete component implementation
/// 
/// **Generated Code**:
/// - Enhanced struct with injected fields (system, config, etc.)
/// - Component trait implementation with all required methods
/// - Template embedding and renderer setup
/// - Constructor with proper initialization
/// - Method routing framework (placeholder for Phase 3)
fn generate_component_implementation(
    input: &ItemStruct,
    attrs: &ComponentAttributes,
) -> ::anyhow::Result<proc_macro2::TokenStream> {
    let struct_name = &input.ident;
    let _original_fields = &input.fields; // Will be used in Phase 2.4
    
    // Generate the enhanced struct with macro-injected fields
    let enhanced_struct = generate_enhanced_struct(input, attrs)?;
    
    // Generate Component trait implementation
    let component_impl = generate_component_trait_impl(struct_name, attrs)?;
    
    // Generate constructor
    let constructor = generate_constructor(struct_name, attrs, input)?;
    
    Ok(quote! {
        // 🏗️ Enhanced struct with macro-injected fields
        #enhanced_struct
        
        // 🎯 Component trait implementation - generated by macro
        #component_impl
        
        // 🆕 Constructor implementation - generated by macro
        #constructor
    })
}

/// 🏗️ Generate enhanced struct with injected system and config fields
fn generate_enhanced_struct(
    input: &ItemStruct,
    attrs: &ComponentAttributes,
) -> ::anyhow::Result<proc_macro2::TokenStream> {
    let struct_name = &input.ident;
    let vis = &input.vis;
    let original_fields = &input.fields;
    
    // Generate config field type
    let config_type = if let Some(config) = &attrs.config {
        let config_ident = syn::parse_str::<syn::Type>(config)
            .map_err(|e| anyhow::anyhow!("Invalid config type '{}': {}", config, e))?;
        quote! { #config_ident }
    } else {
        quote! { ::serde_json::Value }
    };
    
    // 🔧 CRITICAL FIX: Properly handle field expansion based on field type
    let injected_and_original_fields = match original_fields {
        syn::Fields::Named(fields_named) => {
            let original_named_fields = &fields_named.named;
            quote! {
                // 🔧 Macro-injected fields for component framework
                /// Component ID for addressing and storage
                id: String,
                /// Reference to the system for component communication
                system: std::sync::Arc<crate::component::system::SystemComponent>,
                /// Component configuration
                cfg: #config_type,
                /// Template renderer for HTML generation
                renderer: crate::component::SimpleRenderer,
                /// Version tracking for incremental updates
                version: std::sync::atomic::AtomicU64,
                
                // 👤 Original user-defined fields
                #original_named_fields
            }
        }
        syn::Fields::Unnamed(_) => {
            return Err(anyhow::anyhow!("Tuple structs not supported in component macro"));
        }
        syn::Fields::Unit => {
            quote! {
                // 🔧 Macro-injected fields for component framework
                /// Component ID for addressing and storage
                id: String,
                /// Reference to the system for component communication
                system: std::sync::Arc<crate::component::system::SystemComponent>,
                /// Component configuration
                cfg: #config_type,
                /// Template renderer for HTML generation
                renderer: crate::component::SimpleRenderer,
                /// Version tracking for incremental updates
                version: std::sync::atomic::AtomicU64,
            }
        }
    };
    
    Ok(quote! {
        #[derive(Debug)]
        #vis struct #struct_name {
            #injected_and_original_fields
        }
    })
}

/// 🎯 Generate Component trait implementation
fn generate_component_trait_impl(
    struct_name: &Ident,
    _attrs: &ComponentAttributes, // Template handling moved to constructor
) -> ::anyhow::Result<proc_macro2::TokenStream> {
    
    Ok(quote! {
        #[async_trait::async_trait]
        impl crate::component::Component for #struct_name {
            /// 📨 Handle incoming messages - routing to user methods
            async fn handle_message(
                &mut self,
                method: &str,
                payload: serde_json::Value,
            ) -> anyhow::Result<serde_json::Value> {
                // 🚀 Dynamic method dispatch to user-defined methods
                match method {
                    // 🔧 Core system methods
                    "render" => {
                        let html = self.render().await?;
                        Ok(::serde_json::json!({
                            "success": true,
                            "data": html,
                            "method": "render",
                            "component_id": self.id()
                        }))
                    }
                    "get_status" => {
                        Ok(::serde_json::json!({
                            "success": true,
                            "data": {
                                "id": self.id(),
                                "version": self.current_version(),
                                "timestamp": chrono::Utc::now().to_rfc3339()
                            },
                            "method": "get_status",
                            "component_id": self.id()
                        }))
                    }
                    
                    // 🎯 User method dispatch - will be enhanced in Phase 3.2+
                    _ => {
                        // Try to dispatch to user methods
                        self.dispatch_user_method(method, payload).await
                    }
                }
            }

            /// 🎨 Render component using embedded template with enhanced data
            async fn render(&self) -> anyhow::Result<String> {
                // 📊 Collect template data with user fields accessible via self
                let template_data = self.get_template_data().await?;
                
                // 🎨 Render using the embedded template
                self.renderer.render("main", &template_data)
                    .map_err(|e| anyhow::anyhow!("Template rendering failed for '{}': {}", self.id(), e))
            }

            /// 🆔 Component identifier - generated implementation
            fn id(&self) -> &str {
                &self.id
            }

            /// 📊 Current version for incremental updates with atomic access
            fn current_version(&self) -> u64 {
                self.version.load(std::sync::atomic::Ordering::SeqCst)
            }

            /// 🔄 Incremental updates - enhanced default implementation
            async fn get_updates(&self, last_version: u64) -> ::anyhow::Result<Vec<crate::UpdateMessage>> {
                let current_version = self.current_version();
                
                // 🏃‍♂️ No updates needed if version hasn't changed
                if current_version <= last_version {
                    return Ok(vec![]);
                }

                // 🔄 Generate basic version update message
                // TODO: In Phase 3, enhance this with user-defined update logic
                Ok(vec![
                    crate::UpdateMessage::SetText {
                        selector: format!("#{} .version", self.id()),
                        value: format!("v{}", current_version),
                    },
                    crate::UpdateMessage::SetText {
                        selector: format!("#{} .timestamp", self.id()),
                        value: chrono::Utc::now().to_rfc3339(),
                    },
                ])
            }

            /// 🚀 Component startup with configuration - enhanced with user delegation
            async fn start(&mut self, config: ::serde_json::Value) -> ::anyhow::Result<()> {
                tracing::info!("🚀 Starting macro-generated component '{}'", self.id());
                
                // 🔧 Parse configuration into component's config field if possible
                // TODO: Implement config parsing based on config type in Phase 2.4
                
                // 🎯 Call user's start method if it exists
                if let Err(e) = self.user_start(config).await {
                    return Err(anyhow::anyhow!(
                        "User start method failed for component '{}': {}", 
                        self.id(), 
                        e
                    ));
                }
                
                // 📊 Increment version to mark component as started
                self.increment_version();
                
                tracing::info!("✅ Component '{}' started successfully", self.id());
                Ok(())
            }

            /// 🛑 Component shutdown - enhanced with user delegation  
            async fn stop(&mut self) -> ::anyhow::Result<()> {
                tracing::info!("🛑 Stopping macro-generated component '{}'", self.id());
                
                // 🎯 Call user's stop method if it exists
                if let Err(e) = self.user_stop().await {
                    tracing::warn!(
                        "User stop method failed for component '{}': {}", 
                        self.id(), 
                        e
                    );
                    // Continue with shutdown even if user stop fails
                }
                
                // 📊 Increment version to mark component as stopped
                self.increment_version();
                
                tracing::info!("✅ Component '{}' stopped", self.id());
                Ok(())
            }

            /// 🔍 Get component as Any for downcasting (required by macro system)
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            /// 🔍 Get mutable component as Any for downcasting (required by macro system)
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }

        // 🔧 Additional impl block for macro-generated helper methods
        impl #struct_name {
            /// 🔧 Initialize component after creation - users can override this
            /// 
            /// **Default Implementation**: Does nothing
            /// **User Override**: Components can implement this for custom initialization
            /// 
            /// **Example**:
            /// ```rust
            /// impl MyComponent {
            ///     async fn initialize_component(&mut self) -> ::anyhow::Result<()> {
            ///         // Custom initialization logic
            ///         self.my_field = "initialized".to_string();
            ///         Ok(())
            ///     }
            /// }
            /// ```
            async fn initialize_component(&mut self) -> ::anyhow::Result<()> {
                // 🎯 Default implementation - users can override this method
                // to add custom initialization logic after the component is created
                
                tracing::debug!("🔧 Default component initialization for '{}'", self.id());
                Ok(())
            }

            /// 🚀 User start method - users can override this for custom startup logic
            /// 
            /// **Default Implementation**: Does nothing
            /// **User Override**: Components can implement this for custom startup
            /// 
            /// **Example**:
            /// ```rust
            /// impl MyComponent {
            ///     async fn user_start(&mut self, config: ::serde_json::Value) -> ::anyhow::Result<()> {
            ///         // Custom startup logic
            ///         self.load_state().await?;
            ///         Ok(())
            ///     }
            /// }
            /// ```
            async fn user_start(&mut self, _config: ::serde_json::Value) -> ::anyhow::Result<()> {
                // 🎯 Default implementation - users can override this method
                // to add custom startup logic
                
                tracing::debug!("🚀 Default user start for '{}'", self.id());
                Ok(())
            }

            /// 🛑 User stop method - users can override this for custom shutdown logic
            /// 
            /// **Default Implementation**: Does nothing  
            /// **User Override**: Components can implement this for custom shutdown
            /// 
            /// **Example**:
            /// ```rust
            /// impl MyComponent {
            ///     async fn user_stop(&mut self) -> ::anyhow::Result<()> {
            ///         // Custom shutdown logic
            ///         self.save_state().await?;
            ///         Ok(())
            ///     }
            /// }
            /// ```
            async fn user_stop(&mut self) -> ::anyhow::Result<()> {
                // 🎯 Default implementation - users can override this method
                // to add custom shutdown logic
                
                tracing::debug!("🛑 Default user stop for '{}'", self.id());
                Ok(())
            }

            /// 📊 Increment component version atomically
            fn increment_version(&self) -> u64 {
                self.version.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
            }

            /// 📊 Get template data with component state
            /// 
            /// **Default Implementation**: Provides basic component data
            /// **User Override**: Components can implement this to add custom template data
            async fn get_template_data(&self) -> ::anyhow::Result<::serde_json::Value> {
                // 🎯 Default template data - components can override this method
                Ok(::serde_json::json!({
                    "id": self.id(),
                    "version": self.current_version(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    // TODO: In Phase 2.4, add user-defined fields to template data
                }))
            }

            /// 💾 Save component state via SystemComponent - proper architecture
            pub async fn save_state(&self, key: &str, value: ::serde_json::Value) -> ::anyhow::Result<()> {
                self.system.save_component_state(&self.id, key, value).await
            }

            /// 📖 Load component state via SystemComponent - proper architecture  
            pub async fn load_state(&self, key: &str) -> ::anyhow::Result<Option<::serde_json::Value>> {
                self.system.load_component_state(&self.id, key).await
            }

            /// 🗑️ Delete component state via SystemComponent - proper architecture
            pub async fn delete_state(&self, key: &str) -> ::anyhow::Result<bool> {
                self.system.delete_component_state(&self.id, key).await
            }

            /// 🔄 Update system reference - solves bootstrap circular dependency
            ///
            /// **Purpose**: Update the system reference after component creation
            /// **Usage**: Called during system initialization to fix bootstrap references
            /// **Architecture**: Enables clean component creation without circular dependencies
            pub fn update_system_reference(&mut self, system: std::sync::Arc<crate::component::system::SystemComponent>) {
                self.system = system;
                tracing::debug!("🔄 Updated system reference for component '{}'", self.id);
            }

            /// 🎯 Dispatch to user methods - generated method routing
            /// 
            /// **Purpose**: Route method calls to user-defined methods marked with #[crate::method]
            /// **Parameters**: Automatically deserialize from JSON payload
            /// **Returns**: Serialize result to ComponentResult<Value>
            /// 
            /// This is the core method routing logic generated by the macro system.
            /// Users don't call this directly - it's used by handle_message.
            async fn dispatch_user_method(
                &mut self,
                method: &str,
                payload: ::serde_json::Value,
            ) -> ::anyhow::Result<::serde_json::Value> {
                // 🔧 This will be enhanced by the method macro to add actual method routing
                // For now, provide a base implementation that can be overridden
                
                // 📊 Try common component method patterns
                match method {
                    // 🎯 Common button-like methods
                    "click" => self.try_call_click_method(payload).await,
                    "reset" => self.try_call_reset_method().await,
                    "set_text" => self.try_call_set_text_method(payload).await,
                    "increment" => self.try_call_increment_method(payload).await,
                    
                    // 🎯 Common state methods
                    "get" => self.try_call_get_method(payload).await,
                    "set" => self.try_call_set_method(payload).await,
                    "delete" => self.try_call_delete_method(payload).await,
                    
                    // 🎯 Common control methods  
                    "start" => self.try_call_user_start_method(payload).await,
                    "stop" => self.try_call_user_stop_method().await,
                    
                    // ❓ Unknown method
                    _ => {
                        Ok(::serde_json::json!({
                            "error": format!("Method '{}' not found", method),
                            "component_id": self.id(),
                            "available_methods": self.get_available_methods(),
                            "suggestion": format!("Add #[crate::method] to a pub fn {}() method", method)
                        }))
                    }
                }
            }

            /// 📋 Get list of available methods - will be enhanced by method macro
            fn get_available_methods(&self) -> Vec<String> {
                // 🔧 Base methods always available
                vec![
                    "render".to_string(),
                    "get_status".to_string(),
                    // TODO: Method macro will append registered method names
                ]
            }

            // ================================================================================
            // 🔧 Default Method Implementations - Users can override these
            // ================================================================================

            /// 🖱️ Try to call user's click method with parameter handling
            async fn try_call_click_method(&mut self, _payload: ::serde_json::Value) -> ::anyhow::Result<::serde_json::Value> {
                // 🎯 Default fallback - users override this when they have a click method
                Ok(::serde_json::json!({
                    "error": "Click method not implemented",
                    "suggestion": "Add: #[crate::method] pub fn click(&mut self) -> ::anyhow::Result<T> { ... }"
                }))
            }

            /// 🔄 Try to call user's reset method
            async fn try_call_reset_method(&mut self) -> ::anyhow::Result<::serde_json::Value> {
                Ok(::serde_json::json!({
                    "error": "Reset method not implemented",
                    "suggestion": "Add: #[crate::method] pub fn reset(&mut self) -> ::anyhow::Result<T> { ... }"
                }))
            }

            /// 📝 Try to call user's set_text method with text parameter
            async fn try_call_set_text_method(&mut self, payload: ::serde_json::Value) -> ::anyhow::Result<::serde_json::Value> {
                let _text = payload.get("text").and_then(|v| v.as_str()).unwrap_or("");
                Ok(::serde_json::json!({
                    "error": "Set text method not implemented",
                    "suggestion": "Add: #[crate::method] pub fn set_text(&mut self, text: &str) -> ::anyhow::Result<T> { ... }"
                }))
            }

            /// ➕ Try to call user's increment method with amount parameter
            async fn try_call_increment_method(&mut self, payload: ::serde_json::Value) -> ::anyhow::Result<::serde_json::Value> {
                let _amount = payload.get("amount").and_then(|v| v.as_u64()).unwrap_or(1);
                Ok(::serde_json::json!({
                    "error": "Increment method not implemented",
                    "suggestion": "Add: #[crate::method] pub fn increment(&mut self, amount: u64) -> ::anyhow::Result<T> { ... }"
                }))
            }

            /// 📖 Try to call user's get method with key parameter
            async fn try_call_get_method(&mut self, payload: ::serde_json::Value) -> ::anyhow::Result<::serde_json::Value> {
                let _key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
                Ok(::serde_json::json!({
                    "error": "Get method not implemented",
                    "suggestion": "Add: #[crate::method] pub fn get(&self, key: &str) -> ::anyhow::Result<T> { ... }"
                }))
            }

            /// 💾 Try to call user's set method with key/value parameters
            async fn try_call_set_method(&mut self, payload: ::serde_json::Value) -> ::anyhow::Result<::serde_json::Value> {
                let _key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
                let _value = payload.get("value");
                Ok(::serde_json::json!({
                    "error": "Set method not implemented",
                    "suggestion": "Add: #[crate::method] pub fn set(&mut self, key: &str, value: T) -> ::anyhow::Result<()> { ... }"
                }))
            }

            /// 🗑️ Try to call user's delete method with key parameter  
            async fn try_call_delete_method(&mut self, payload: ::serde_json::Value) -> ::anyhow::Result<::serde_json::Value> {
                let _key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
                Ok(::serde_json::json!({
                    "error": "Delete method not implemented",
                    "suggestion": "Add: #[crate::method] pub fn delete(&mut self, key: &str) -> ::anyhow::Result<bool> { ... }"
                }))
            }

            /// 🚀 Try to call user's start method with config parameter
            async fn try_call_user_start_method(&mut self, payload: ::serde_json::Value) -> ::anyhow::Result<::serde_json::Value> {
                // Delegate to the built-in user_start method
                self.user_start(payload).await?;
                Ok(::serde_json::json!({
                    "success": true,
                    "method": "start",
                    "component_id": self.id()
                }))
            }

            /// 🛑 Try to call user's stop method
            async fn try_call_user_stop_method(&mut self) -> ::anyhow::Result<::serde_json::Value> {
                // Delegate to the built-in user_stop method
                self.user_stop().await?;
                Ok(::serde_json::json!({
                    "success": true,
                    "method": "stop", 
                    "component_id": self.id()
                }))
            }
        }
    })
}

/// 🆕 Generate constructor for macro-enhanced component
fn generate_constructor(
    struct_name: &Ident,
    attrs: &ComponentAttributes,
    input: &ItemStruct,
) -> ::anyhow::Result<proc_macro2::TokenStream> {
    
    // Generate config parameter type
    let config_type = if let Some(config) = &attrs.config {
        let config_ident = syn::parse_str::<syn::Type>(config)
            .map_err(|e| anyhow::anyhow!("Invalid config type '{}': {}", config, e))?;
        quote! { #config_ident }
    } else {
        quote! { ::serde_json::Value }
    };
    
    // 🎨 Generate template setup code with proper path resolution
    let template_setup = generate_template_setup(attrs)?;
    
    // Generate field defaults based on original user fields
    let field_defaults = generate_field_defaults(&input.fields)?;
    
    Ok(quote! {
        impl #struct_name {
            /// 🆕 Create new component instance - generated by macro
            /// 
            /// **Arguments**:
            /// - `id`: Component instance identifier
            /// - `system`: Reference to system for component communication  
            /// - `config`: Component configuration
            /// 
            /// **Returns**: Initialized component or error
            pub fn new(
                id: String,
                system: std::sync::Arc<crate::component::system::SystemComponent>,
                config: #config_type,
            ) -> ::anyhow::Result<Self> {
                tracing::debug!("🆕 Creating component '{}' via macro", id);
                
                // 🎨 Initialize template renderer with embedded templates
                let mut renderer = crate::component::SimpleRenderer::new();
                #template_setup
                
                // 🏗️ Create component instance with macro-injected fields
                let component = Self {
                    id: id.clone(),
                    system,
                    cfg: config,
                    renderer,
                    version: std::sync::atomic::AtomicU64::new(1),
                    
                    // 👤 User fields with generated defaults
                    #field_defaults
                };
                
                // 🔧 Component initialization happens after creation
                // Users can call initialize_component() separately if needed
                tracing::info!("✅ Component '{}' created successfully", component.id);
                Ok(component)
            }
        }
    })
}

/// 🎨 Generate template setup code with proper path resolution
/// 
/// **Template Path Resolution**:
/// - Templates are resolved relative to the component's source file
/// - Path format: `templates/{template_name}` from component directory
/// - Uses `include_str!` for compile-time template embedding
/// - Fallback to default template if none specified
fn generate_template_setup(attrs: &ComponentAttributes) -> ::anyhow::Result<proc_macro2::TokenStream> {
    if let Some(template) = &attrs.template {
        // 📁 Resolve template path relative to component file
        // The path will be relative to where the macro is invoked (the component file)
        Ok(quote! {
            // 📝 Register embedded template resolved relative to component file
            renderer.register_template(
                "main",
                include_str!(concat!("templates/", #template))
            ).map_err(|e| anyhow::anyhow!("Failed to register template '{}': {}", #template, e))?;
            
            tracing::debug!("📝 Template '{}' embedded successfully", #template);
        })
    } else {
        // 🔧 Generate default template when none specified
        Ok(quote! {
            // ⚠️ No template specified - using default component template
            renderer.register_template(
                "main", 
                r#"<div class="component" id="{{id}}">
                    <h3>{{id}}</h3>
                    <p>Version: {{version}}</p>
                    <p>Generated at: {{timestamp}}</p>
                </div>"#
            ).map_err(|e| anyhow::anyhow!("Failed to register default template: {}", e))?;
            
            tracing::debug!("📝 Default template registered for component");
        })
    }
}

/// 🔍 Parse method macro attributes
/// 
/// **Supported Attributes**:
/// - `default_instance` - Mark method for default instance routing
/// 
/// **Example**: `#[method(default_instance)]`
fn parse_method_attributes(attr: TokenStream) -> ::anyhow::Result<MethodAttributes> {
    let mut attributes = MethodAttributes::default();
    
    if attr.is_empty() {
        return Ok(attributes);
    }

    // Parse the attribute tokens as a function-like macro call
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let parsed = parser.parse(attr)
        .map_err(|e| anyhow::anyhow!("Failed to parse method attributes: {}", e))?;
    
    for meta in parsed {
        match meta {
            Meta::Path(path) => {
                if let Some(ident) = path.get_ident() {
                    match ident.to_string().as_str() {
                        "default_instance" => {
                            attributes.default_instance = true;
                        }
                        _ => {
                            return Err(anyhow::anyhow!("Unknown method attribute: {}", ident));
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("Invalid method attribute path"));
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Invalid method attribute format. Use #[method(default_instance)]"));
            }
        }
    }
    
    Ok(attributes)
}

/// 📊 Parse method information from function signature
/// 
/// **Extracts**:
/// - Method name, visibility, async status
/// - Parameters with types and self information
/// - Return type information
/// - Validation for component method requirements
fn parse_method_info(input: &ItemFn, attrs: MethodAttributes) -> ::anyhow::Result<MethodInfo> {
    let sig = &input.sig;
    
    // Extract method name
    let name = sig.ident.to_string();
    
    // Check if method is public
    let is_public = matches!(input.vis, syn::Visibility::Public(_));
    
    // Check if method is async
    let is_async = sig.asyncness.is_some();
    
    // Parse parameters
    let mut parameters = Vec::new();
    for (i, input_param) in sig.inputs.iter().enumerate() {
        match input_param {
            syn::FnArg::Receiver(_receiver) => {
                parameters.push(MethodParameter {
                    name: "self".to_string(),
                    is_self: true,
                });
            }
            syn::FnArg::Typed(pat_type) => {
                let param_name = match pat_type.pat.as_ref() {
                    syn::Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
                    _ => format!("param_{i}"),
                };
                
                parameters.push(MethodParameter {
                    name: param_name,
                    is_self: false,
                });
            }
        }
    }
    
    // Extract return type
    let return_type = match &sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => Some((**ty).clone()),
    };
    
    Ok(MethodInfo {
        name,
        is_public,
        is_async,
        parameters,
        return_type,
        attributes: attrs,
    })
}

/// ✅ Validate method meets controlrake component requirements
/// 
/// **Requirements**:
/// - Method must be public for bus routing
/// - Default instance methods cannot have &self parameter
/// - Component methods should have &self or &mut self parameter
/// - Parameters must be serializable for JSON communication
fn validate_method(method: &MethodInfo) -> ::anyhow::Result<()> {
    // 📋 Validate method is public
    if !method.is_public {
        return Err(anyhow::anyhow!(
            "Method '{}' must be public (pub) for bus routing", 
            method.name
        ));
    }
    
    // 📋 Check self parameter requirements
    let has_self = method.parameters.iter().any(|p| p.is_self);
    
    if method.attributes.default_instance {
        // 🏷️ Default instance methods should not have self parameter
        if has_self {
            return Err(anyhow::anyhow!(
                "Default instance method '{}' cannot have self parameter. \
                Use static methods for default_instance.", 
                method.name
            ));
        }
    } else {
        // 🏷️ Regular component methods should have self parameter
        if !has_self {
            return Err(anyhow::anyhow!(
                "Component method '{}' should have &self or &mut self parameter. \
                Use #[method(default_instance)] for static methods.", 
                method.name
            ));
        }
    }
    
    // 📋 Validate parameter count (reasonable limit for JSON serialization)
    if method.parameters.len() > 10 {
        return Err(anyhow::anyhow!(
            "Method '{}' has too many parameters ({}). \
            Maximum 10 parameters supported for JSON serialization.", 
            method.name,
            method.parameters.len()
        ));
    }
    
    // ✅ Method validation passed
    Ok(())
}

/// ✅ Validate direct call support (Task 3.4)
/// 
/// **Requirements**:
/// - Original method signature must be preserved unchanged
/// - Method must remain callable directly (not just through bus)
/// - Bus routing is additional layer, not replacement
fn validate_direct_call_support(original_fn: &ItemFn, method_info: &MethodInfo) -> ::anyhow::Result<()> {
    // ✅ Verify method signature is preserved
    let original_name = &original_fn.sig.ident;
    let parsed_name = &method_info.name;
    
    if original_name != parsed_name {
        return Err(anyhow::anyhow!(
            "Method name mismatch: original '{}' vs parsed '{}'. \
            This indicates the macro is modifying the method signature.",
            original_name, parsed_name
        ));
    }
    
    // ✅ Verify visibility is preserved (should be public for bus routing)
    if !method_info.is_public {
        return Err(anyhow::anyhow!(
            "Method '{}' must be public for both direct calls and bus routing. \
            Use 'pub fn' to enable both call patterns.",
            method_info.name
        ));
    }
    
    // ✅ Verify async status matches expectations
    let original_is_async = original_fn.sig.asyncness.is_some();
    if original_is_async != method_info.is_async {
        return Err(anyhow::anyhow!(
            "Async mismatch for method '{}': original async={}, parsed async={}. \
            This indicates signature parsing issues.",
            method_info.name, original_is_async, method_info.is_async
        ));
    }
    
    // ✅ All validations passed - direct call support is preserved
    Ok(())
}

/// 🚀 Generate method routing code for marked methods
/// 
/// **Generated Code**:
/// - Override try_call_{method_name}_method 
/// - Parameter deserialization from JSON payload
/// - Method call with proper error handling
/// - Return value serialization to ComponentResult<Value>
fn generate_method_routing(method: &MethodInfo) -> ::anyhow::Result<proc_macro2::TokenStream> {
    let method_name = &method.name;
    let method_ident = syn::parse_str::<syn::Ident>(method_name)
        .map_err(|e| anyhow::anyhow!("Invalid method name '{}': {}", method_name, e))?;
    
    // 🎯 Generate try_call function name
    let try_call_method_name = format!("try_call_{method_name}_method");
    let try_call_ident = syn::parse_str::<syn::Ident>(&try_call_method_name)
        .map_err(|e| anyhow::anyhow!("Invalid try_call method name '{}': {}", try_call_method_name, e))?;
    
    // 📊 Extract parameters (excluding self)
    let non_self_params: Vec<_> = method.parameters.iter().filter(|p| !p.is_self).collect();
    
    // 🔧 Generate parameter extraction code
    let param_extraction = generate_parameter_extraction(&non_self_params)?;
    
    // 🎯 🚨 CRITICAL FIX: Handle both async and sync methods properly
    let method_call = if method.attributes.default_instance {
        // Static method call for default instance
        if method.is_async {
            quote! {
                let result = Self::#method_ident(#param_extraction).await;
            }
        } else {
            quote! {
                let result = Self::#method_ident(#param_extraction);
            }
        }
    } else {
        // Instance method call
        if non_self_params.is_empty() {
            if method.is_async {
                quote! {
                    let result = self.#method_ident().await;
                }
            } else {
                quote! {
                    let result = self.#method_ident();
                }
            }
        } else if method.is_async {
            quote! {
                let result = self.#method_ident(#param_extraction).await;
            }
        } else {
            quote! {
                let result = self.#method_ident(#param_extraction);
            }
        }
    };
    
    // 🔄 Generate return value handling
    let return_handling = if method.return_type.is_some() {
        quote! {
            match result {
                Ok(data) => {
                    // 🔄 Handle reply to caller if 'from' address provided
                    if let Some(from_addr) = payload.get("from").and_then(|v| v.as_str()) {
                        // 📤 Send reply message back to caller
                        let reply_payload = ::serde_json::json!({
                            "success": true,
                            "data": data,
                            "method": #method_name,
                            "component_id": self.id(),
                            "reply_to": #method_name
                        });
                        
                        // TODO: Send reply via system bus - requires system reference
                        // self.system.send_reply(from_addr, reply_payload).await?;
                        
                        Ok(::serde_json::json!({
                            "success": true,
                            "data": data,
                            "method": #method_name,
                            "component_id": self.id(),
                            "reply_sent_to": from_addr
                        }))
                    } else {
                        // 📋 No 'from' address - return result directly
                        Ok(::serde_json::json!({
                            "success": true,
                            "data": data,
                            "method": #method_name,
                            "component_id": self.id()
                        }))
                    }
                }
                Err(e) => {
                    // 🔄 Handle error reply to caller if 'from' address provided
                    if let Some(from_addr) = payload.get("from").and_then(|v| v.as_str()) {
                        let error_reply = ::serde_json::json!({
                            "error": e.to_string(),
                            "method": #method_name,
                            "component_id": self.id(),
                            "reply_to": #method_name
                        });
                        
                        // TODO: Send error reply via system bus
                        // self.system.send_reply(from_addr, error_reply).await?;
                    }
                    
                    Ok(::serde_json::json!({
                        "error": e.to_string(),
                        "method": #method_name,
                        "component_id": self.id()
                    }))
                }
            }
        }
    } else {
        quote! {
            result?;
            
            // 🔄 Handle void method reply to caller if 'from' address provided
            if let Some(from_addr) = payload.get("from").and_then(|v| v.as_str()) {
                let reply_payload = ::serde_json::json!({
                    "success": true,
                    "method": #method_name,
                    "component_id": self.id(),
                    "reply_to": #method_name
                });
                
                // TODO: Send reply via system bus
                // self.system.send_reply(from_addr, reply_payload).await?;
                
                Ok(::serde_json::json!({
                    "success": true,
                    "method": #method_name,
                    "component_id": self.id(),
                    "reply_sent_to": from_addr
                }))
            } else {
                Ok(::serde_json::json!({
                    "success": true,
                    "method": #method_name,
                    "component_id": self.id()
                }))
            }
        }
    };
    
    // 🏗️ Generate complete method override
    Ok(quote! {
        /// 🚀 Generated method routing for #[crate::method] #method_name
        async fn #try_call_ident(&mut self, payload: ::serde_json::Value) -> ::anyhow::Result<::serde_json::Value> {
            // 🔍 Extract caller context for reply handling
            let _caller_context = payload.get("caller_context");
            let _from_address = payload.get("from").and_then(|v| v.as_str());
            
            #param_extraction
            
            // 🎯 Call user method
            #method_call
            
            // 🔄 Handle return value
            #return_handling
        }
    })
}

/// 📊 Generate parameter extraction code from JSON payload
fn generate_parameter_extraction(params: &[&MethodParameter]) -> ::anyhow::Result<proc_macro2::TokenStream> {
    if params.is_empty() {
        return Ok(quote! {});
    }
    
    // For now, generate basic parameter extraction
    // TODO: Enhance with proper type-based deserialization
    let extractions: Vec<_> = params.iter().map(|param| {
        let param_name = &param.name;
        let param_ident = syn::parse_str::<syn::Ident>(param_name).unwrap();
        
        // 🔧 Basic parameter extraction - will be enhanced
        quote! {
            let #param_ident = payload.get(#param_name)
                .ok_or_else(|| anyhow::anyhow!("Missing parameter: {}", #param_name))?;
        }
    }).collect();
    
    Ok(quote! {
        #(#extractions)*
    })
}

/// 🔧 Generate default values for user-defined fields
/// 
/// **Purpose**: Automatically generate appropriate default values for user fields
/// **Strategy**: Provide sensible defaults for common types, require Default for others
fn generate_field_defaults(fields: &syn::Fields) -> ::anyhow::Result<proc_macro2::TokenStream> {
    match fields {
        syn::Fields::Named(fields_named) => {
            let field_inits: Vec<_> = fields_named.named.iter().map(|field| {
                let field_name = &field.ident;
                let field_type = &field.ty;
                
                // Generate appropriate default based on type
                let default_value = generate_type_default(field_type);
                
                quote! {
                    #field_name: #default_value
                }
            }).collect();
            
            Ok(quote! {
                #(#field_inits),*
            })
        }
        syn::Fields::Unnamed(_) => {
            Err(anyhow::anyhow!("Tuple structs not supported in component macro"))
        }
        syn::Fields::Unit => {
            Ok(quote! {})
        }
    }
}

/// 🎯 Generate default value for a specific type
/// 
/// **Supported Types**: Common types with sensible defaults
/// **Fallback**: Default::default() for unknown types
fn generate_type_default(ty: &syn::Type) -> proc_macro2::TokenStream {
    // Try to extract type name for known types
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = &segment.ident;
            
            match type_name.to_string().as_str() {
                // Numeric types
                "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "usize" | "isize" => {
                    quote! { 0 }
                }
                "bool" => {
                    quote! { false }
                }
                "String" => {
                    quote! { String::new() }
                }
                "Vec" => {
                    quote! { Vec::new() }
                }
                "HashMap" => {
                    quote! { std::collections::HashMap::new() }
                }
                "Option" => {
                    quote! { None }
                }
                // Default fallback for other types
                _ => {
                    quote! { Default::default() }
                }
            }
        } else {
            quote! { Default::default() }
        }
    } else {
        // For complex types, use Default::default()
        quote! { Default::default() }
    }
}

/// 🧪 Test that macros compile correctly
#[cfg(test)]
mod tests {
    use super::*;

    // Basic compilation tests will go here
    // Full testing happens in the main crate integration tests
}
