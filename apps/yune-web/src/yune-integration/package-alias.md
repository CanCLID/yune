# Local Package Alias for @yune-ime/yune-web-runtime

`yune-web` imports `@yune-ime/yune-web-runtime` from the repository-owned package
without publishing it to npm.

The app root declares the local dependency directly:

```json
{
  "dependencies": {
    "@yune-ime/yune-web-runtime": "file:../../packages/yune-web-runtime"
  }
}
```

Install from the app root:

```bash
npm --prefix apps/yune-web install
```

Before building the app, make sure the local runtime package still builds:

```bash
npm --prefix packages/yune-web-runtime run build
```
