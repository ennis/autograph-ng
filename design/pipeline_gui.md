Idea: GUI for graphics pipeline configuration

- Web UI (react?)
- simplified shaders
- export to several formats
- show explicit pipeline formulas
	- do not show the blend mode name, but show the final blend mode formula
	- depth test, stencil test
- add interpolants, rasterizer config
- blend targets

#### Ideas for GUIs in Rust
- no CSS (use hot-reloadable rust)
- allocate objects in arena
    - parent-child in pointers
    - drop whole arena every time?
        - very wasteful
- if no objects kept by application between frames, then need a way to map from ID to pointer
    - hashmap
- Is immediate mode the best API?
    - conditional rendering, frequent changes
    - but respecification of the whole UI, ID's, hashmaps
- Concept: components render() small snippets of UI tree, but does not expand its children
    - match the rendered snippet with the existing tree, starting from the anchor
    - orphan any unreferenced children
    - then render() children (can reorder children)
- Tree structure
    - low memory usage
    - optimize for large number of children
    - one arena for nodes
        - and Vecs for child lists
    - mutate children easily
    - removing children: ???
- minimize memory allocations per-component
    - ideally: no per-component heap allocations (stacks & arenas are fine)
- Optimize for the most common case: full respec, no changes
- Solution:
    - keep current id_tree
    - but remove "virtual DOM": update the UI tree piecewise

#### GUI rendering
- webrender?
    - maybe, but cannot change backend (stuck with OpenGL)
- custom stuff on top of autograph-render?
    - lyon + font-kit ?
        - +autograph-render
    - native 2D?
        - piet?
        - issues with interop
    - ideally: 
        - draw stuff using the native 2D API
            - Direct2D / GDI on windows
            - whatever on macOS
            - whatever on linux
        - draw text using the native text rendering API
            - DirectWrite on windows
            - whatever on macOS
            - freetype (?) on linux
        - somehow blend that on top of the scene (how?)
        - interop somehow 

#### Separation of concerns
- UI tree update, input handling, geometry and rendering are in separate places
    - input: needs geometry + ui tree
    - rendering: needs geom + ui tree
    - update: needs ui tree
- focus on incremental UI tree updates (no geometry, etc.) from an IMGUI-like (or React-like) interface
    - could use geometry for optimizations    
