// Ambient declarations for the game: Vite's client types give `import.meta.env` (where `DEV` is true in a
// development build, and the developer tools load only then). The stylesheets are imported for their side effect
// only, so every `.css` import is a module with no exports.

/// <reference types="vite/client" />

declare module "*.css";
