1. here is something called version in macros. Kill it completely. increment_version is weird.
1.1. update all components must be performed automatically, when method was called and have result.
2. no need to save last_updated into db.
3. put into serde default u32(0)