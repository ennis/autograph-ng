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

#### About rendering
- Need something native, that handles every aspect of UI
    - text
    - shape rendering
    - effects
- and on top of that, something that provides integration with the OS
    - native menus
    - native text boxes?
- Text being the most complex thing

#### Experience with WPF
- a visual designer is nice to have
- ...but data binding is incredibly cumbersome
    - all dynamic properties must be wrapped in a DependencyProperty manually
    - yet, it allows piecewise update of affected visual elements
- data templates are good
    - function that returns a visual tree
- control templates?
    - allow reuse of logic by using the same control with a different visual tree
    - powerful styling (can draw anything)
    - to consider

- Q: Logic reuse
- Q: Restyle but keep logic

- Example of control templates: Button
    - main element defines states (Hover, Main, Disabled, Clicked)
    - template is a function from states to visual tree
        - template can be changed, defaults to default visual style
            - template can produce elements with state 
        - NOT directly a renderer: generates a visual tree that can be layout
        - can have more than one template for an element 
            - e.g. different parts
    - issue: deciding when the template must be re-instantiated
        - when arguments to the template change 
            - but arguments may be unused
        - react to input events
            - in event handler, r/w access to state of element AND ref to parent element, optionally
            - *event handlers can produce new elements*
                - hard
            - alternative: "non-local events" are handled via the immediate path
                - in the immediate path, state should be accessible mutably
            - also: state may be modified externally
                - e.g. a task that animates a property
                - need to keep a ref to the element...
        - how does react (the framework) handle state?
- Structure as data is also good (XAML, FXML)
    - Macro?
    - Will generate code
- Styling?
    - No CSS, use a type-safe alternative
    - 

- Node graph control
    - Tree:
        - Node container
            - Node
                - Inputs
                    - Input A
                        - Circle
                            - <event> -> Graph.connect_drag_event()     // access to state outside element
                            - state: RefCell<EndpointState> -> &RefCell<GraphState>
                                - two live mut refs at the same time, must use RefCell
                                - all state in RefCell
                        - Circle 
                            - <event>
                            - ...
                - Outputs
            - Node
                - ...

#### Problem: DO NOT OWN THE STATE / DO NOT OWN THE EVENT LOOP
- assume that the application holds a mutable ref to the state, and that it can change externally
    - input is not the only way to change the state
- this means that the UI only views the state for a short period of time (short borrow)
    - mutation happens predictably in that scope
- state must be considered carefully in complex GUIs
    - e.g. undo/redo -> commands that modify the state
    - `ui.textbox(&mut string)`
        - undo/redo?
        - not for external state...
    - so, widgets can take two things as input:
        - directly a &mut ref to the state (no binding possible)
        - OR a 'smart' state reference to a container (like a binding)
            - implements undo/redo, track changes
            - observable
        
#### Practical considerations
- leverage existing architectures that don't look too bad
    - flutter?
- don't write a renderer
- salvage text layout from somewhere
    - target window only, and use d2d+dwrite?
- salvage layout algorithms from somewhere
    - not flexbox, though
        - not practical
    - stack + grid + canvas, like wpf

#### Basic idea of react-like frameworks
- UI visuals are a function of state (internal/widget and external/app) 
- Widgets emit events which are handled and modify part of the state

- When state change, the visual tree is rebuilt
    - True IMGUI: assume that the whole state changes on every frame, and handle events at the same time
    - Retained-mode: detect state changes, rebuild only parts that need to be updated

- The problem is the state
    - accessing the state bound to a widget is a computation in itself
    - of course, it might as well just be a reference to the data, but then the widget needs to BORROW the data
         - which is not good, because we want to be able to modify the state externally 
            - removing elements from collections, modifying elements
            - setting the issue of having multiple &mut refs at the same time, modifications may very well invalidate pointers anyway and move data around
    -> the solution here might be calling the UI code multiple times
        - what if the state has changed since last call?
            - will be reflected
    -> another solution is something like a smart reference to an element of state
        - actually, more like a "path"
            - member(name) / index(0) / member(other)
            - composition?
        - actually, more like a command
            - operations on state are fully reversible
    -> must build the visual tree in render, and handle events at the same time
        - what about the contents of a composite (aggregate) component? 
            - are they created, and events handled, before the aggregate component? or after?
                - contents are created _within_ the parent component
                - components must have a *place* to render into
                    - a 'Cursor' points to the place into which the visual should be rendered
                    - there are no "stand-alone" visuals: fragment of visuals not bound to a particular place in the visual tree
                - after rendering stuff into a place, returns a sequence of generated events
        - UI: Internal state + External state + Event -> mutate tree, new states
            - just an event handling mechanism
            - rendering is just another event
            - Bundle cursor + event together
                - If event not handled, then just return the event, and the parent will process it
                - Otherwise, return nothing
            - The whole procedure of event routing is directly apparent in client code
                - no need to learn any underlying behavior
            - verbosity?
                - macros may alleviate some pains
        - Issue: event capture
            - send mouse event to a particular element in the visual tree
                - must traverse everything
        - Issue: the visual tree may be the result of a complex calculation
            - (but hopefully not)
            - wasteful to recompute every time an event is received
                - note that we are not re-rendering: just outputting visuals
            - problem: if event is a callback, then no access to external state
                - that's a *hard rule*: don't borrow or own external state longer than necessary
        - Find a way to make the callback-based method work
            - short-lived borrows
- Idea: model accessors
    - component has (owns) a "view-model" (tentatively named, may be different from MVVM) that takes a root mut ref to the state and produces
        a mut ref to the correct element of state
    - e.g. `list[index].field.field2`
        - take mut ref to list, produce mut ref to field2, but without the need to keep a ref to field2 (since it may have moved in memory, after all)
            - some kind of descriptor object that produces the access
        - Desc(.field2)
    - Basically, a 'weak reference', or 'accessor' that does not borrow the model data
        - weakref.get_mut(root_state) 
            - calls parent_weakref.get_mut(root_state).get_field(field)
        - a bridge between the model (which does not need to be instrumented for modification) and the view
        - e.g. `let accessor = <T as StructModel>::field_access(parent, "field_name")`
            - parent is an accessor that mut-derefs to an instance of T
            - `Accessor::access(&mut root_state)`
        - accessors must be allocated somewhere, and are referenced by child elements
            - sometimes accessors are simply bare functions (e.g. field access) but they can be closures 
                - e.g. access an element in a collection: need to store the index
                - access element in hashmap: needs key
            - boxed
            - no ref to parent: use the tree
        
        
```
@button(label = state.text) {
    on_click = {
        // state context: &mut state
        // every reference to state is replaced with context.state_mut() to signal that the state may be modified 
        state.text = "Button has been clicked" 
    }

    on_release = {
        state.text = "Button has been released"
    }
}

@slider(min = 0.0, max = 1.0) <- state.value        // state binding

slider:
    // parameters
    min: f32;
    max: f32;
    // internal state:
    pos: f32 = 0.0;
    // visuals
    @Grid {
        // default prop is content
        @Rectangle()
    }

@label(text: String)
    @label(state.text[i])       // <- this is not a binding
// some widgets expect state, others just parameters?

issue: a widget is not only a function from state to visual: it's also a way to slice state
-> returns two functions:
    - tree builder / updater
    - event handler
-> logic in rendering?

```

#### Code structure
- code structure has to reflect the way events are processed
```
enter widget {
    capture event processing (fetch events)
    child widgets
    post event processing
}
exit widget
```
- maybe some macros to facilitate

#### A component is not just a function
- combination of:
    - function (event handler)
    - external state type
    - internal state type
    - named properties (named constructor arguments)
    
#### Events:
```
ui.place<T>(id, |ui, state| {

    // pass down event
    let ev = ui.button(ev, "...");
    
    if ev.clicked() {
        
    }

});
```
- trait Component (impl for internal state)
    - external state

#### Hot-reloadability
- why not (just replace code, the model doesn't change)
    - however: very strict interface (cdylib)
        - which is good, I guess
    - view defined in code assumes layout of data in model
        - assume C layout, or whatever
    - better: reflection
        - don't assume layout, bind data model to view via names
            - can use key-value data models
        - auto-derive, of course
        - pass trait object
        - can derive a PHF for fast hash lookups
    - declarative view (no code)
    - views return events?

#### Bonus: cross-language API / programming model

#### Before making a GUI
- do we need a GUI?
    - a text-based format, or even code, may be sufficient
    - also, investigate jetbrains MPS
- do we need a *new* GUI?
    - maybe existing software can be used as the GUI
    - e.g. a generic graph pipeline editor
- do we need a GUI in the same language as the main application?
    - use another GUI package from another language (C++/Qt/QML, Java/JavaFx, C#/WPF...)
- do we need a rust-native GUI?
    - use bindings to existing libraries (ImGui)