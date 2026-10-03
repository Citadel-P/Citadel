export default {
  hooks: {
    onPreParseSchema(schema) {
      // Utoipa emits this redundant constraint for BTreeMap<String, T>. The
      // generator otherwise treats it as a finite set of optional keys.
      if (schema?.propertyNames?.type === 'string' && Object.keys(schema.propertyNames).length === 1) {
        delete schema.propertyNames;
      }
    },
  },
};
