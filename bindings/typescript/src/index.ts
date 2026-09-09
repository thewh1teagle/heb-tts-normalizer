/**
 * Default entry point, and the one the type declarations come from.
 *
 * The browser is the primary target, so this is the browser build. Node resolves the
 * `"node"` condition in `package.json#exports` to `./node.js` instead; both expose the
 * identical surface — `init`, `normalize`, `Normalizer`, `version`, `isReady`.
 */

export * from "./browser.js";
