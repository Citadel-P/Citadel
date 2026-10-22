import createDOMPurify from 'dompurify';

// Monaco installs/removes hooks while rendering hover and completion markup.
// Keep its hooks and configuration separate from other DOMPurify consumers.
export default createDOMPurify();
