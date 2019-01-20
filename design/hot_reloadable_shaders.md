

#### Hot-reloadable shaders:
* Have interface types and shader code in an easily editable and straightforward format.
* Reduce boilerplate to a minimum
	* helper crate gfx2_shaders / shader_prelude
	* contains pub use of Pipeline config structs, re-exports of gfx2_derive and gfx2_shader_macros
	* merge shader_prelude into gfx2::shader_prelude
	* gfx2 re-exports proc macros from derive and shader_macros


#### Beyond: the render engine
* scene
* animations
* curves
* layers

#### Renderer interface simplification
* Remove RendererBackend parametrization, instead of wrapping
	* Or rather, make `Renderer` type-erased
	* Replace with opaque pointer to `dyn Resource`
	* 128-bit resource handles, basically
		* this may have some overhead
		* is it worth it?
	* Gain:
		* hot-reloadable render code
			* can distribute as binary, no need for recompilation
			* less compiled code duplication in hot-reload DLLs
		* switch backends at runtime
		* less syntactical noise in application code (although type aliases reduce it already)
		    * derivation of interfaces
	* Loss:
		* fat pointers (64 -> 128)
		* perf (need downcasts)
		* downcasts:
		    * create descriptor set
		    * command buffers
		    * commands
		    * could perform downcasts in the wrapper, but:
		        * problematic downcasts: large arrays
		        * downcast whole arrays before passing to backend
		* Could a DX12 backend directly impl the Traits on COM interfaces?
		    * with the rust wrapper, COM interfaces are just structs containing a pointer to the vtable
		    * &dyn Image would be:
		        * `data -> vtbl -> [COM function pointers]`
		        * `vtbl -> impl Image`
		    * if possible:
		        * `data -> [COM function pointers]`
		        * `vtbl -> impl Image for VTbl`
		        * just implement the trait on the VTbl or Vtbl wrapper
    * Other option: switch back to raw, usize handles with lifetime tag
        * unsafe cast to pointer type when needed
        * backend never called directly
        * need renderer ref to call methods on objects
        * can squish OpenGL indices into the handles
    * main issue with non-pointer handles: make sure that the handle is of the correct type and belongs to the device
		* multi-device: sharing arenas between devices?
		* ultimate solution: allow only one renderer instance at runtime
		    * multi-device handled in backend
		    * can unchecked_downcast as needed
		        * issue: can forge a Backend object by manually implementing the trait...
    * Q: is there a reasonable scenario that would require instantiating more than one renderer at the same time?
        * multi-window? no -> in backend
        * multi-device? no -> in backend
        * remote rendering?
            * send render commands to another machine
            * maybe...
            * but somewhat useless? would need to duplicate resources
            * don't support multi-GPU across different API...
		
* keep it for now


#### Command buffers
* in backend OR backend-agnostic

#### Next: untyped arenas?
* would simplify management of resources
* arena does not need to be an associated type
* Gain:
	* less code in backend
* Loss:
	* drop_arena() must scan for objects to delete (linear scan?): perf
	* overhead for typedescs
	* complexity of implementation
* for now: stick with backend arenas


#### Hot-reload infrastructure
* Symbols: statics, functions
* In shaders, it makes sense to expose both interface types and SPIR-V bytecode at the same place (within the same module)
	* types are not hot-reloadable
	* functions and data are
	* must detect whether the types have changed: signature?
* issue: any reference to hot-reloadable data should be bound to the lifetime of the dylib
* since the crate contains types, must have an rlib
	* generate hot-reload stubs for some symbols?
* alternative: minimal surface area?
	* dynamically register data

```
#[hot_reload]
#[name="DATA"]
pub const DATA: u32 = 0;

// let lib = lib::hot_reload_stub();
// lib: bunch of exported symbols
//		modules -> structs?
// lib.DATA
// lib.func()
// let blit_shaders = dynamic!(common_shaders::blit);
// blit.VERTEX, blit.FRAGMENT, blit.function...
```

* Issue: generating the struct that represents all dynamically loadable symbols
	* proc_macro on module, #[hot_reload], scan all items with 'extern, no_mangle'
	* proc_macro on root module?
* Issue: 'static lifetimes are not really static: they outlive the lifetime of the dynamic library
	* replace ('downgrade') 'static with 'lib somehow.
	* 'static in output position:
		* `fn() -> 'static` => `fn() -> 'lib`
		* Note: `fn('static) -> 'static` => `fn('lib) -> 'lib` is not a valid transformation
		* `PhantomData<fn(&'lib)>` (`PhantomData<fn(&'lifetime_of_return)>`)
		* Note: `fn() -> &str` is fortunately not considered for lifetime elision
			* must set 'static, which we can replace
		* Issue: `fn() -> Struct`, where Struct has a `&'static str`
			* we are boned: must check types recursively
		* Issue: even with `for<'a> fn(&'a) -> &'a` (no concrete lifetime), the function can still return a static!
			* no way to check without analysis
	* basically, we need to ensure that the returned values do not contain anything that outlive 'lib
		* i.e. 'lib: 'whatever_lifetimes_are_inside_the_returned_value
		* if it doesn't contain references, then OK
		* if it contains a &'static ref then NOT OK: must downgrade
		* if it contains another &'a ref: ensure that 'lib: 'a
			* conclusion: deny &'static
			* bound all output lifetimes with 'lib (`'lib: 'a`)
	* main issue: detect if some type has a &'static ref inside
		* analyze the type tree
	* no easy way to do that
	* instead, mark 'known safe' types with an unsafe trait
		* unsafe trait DynamicLoadSafe
			* implemented for types that are safe to move outside a DLL
			* OR types with a lifetime that can 'downgrade' to the lifetime of the library
			* i.e. `&'static str` is DLL-safe, with `Target=&'lib str`
	* automatic way to convert
		`&'a &'b T where 'b: 'a` to `&'a &'a T`  

```

pub unsafe trait DllSafe<'lib> {
	type Target: 'lib;
	fn downgrade(self, l: &'lib Library) -> Target; 
}

// if we have `for<'a> fn(&'a) -> &'a`, 
// - fix concrete 'a 
// - if 'a outlives 'lib, then can downgrade &'a T to &'lib T
// - if 'a does not outlive 'lib, still OK to call
// - specialization needed?
// 
// Function is callable if:
// - &'a T is DllSafe<'lib>
// 
// Actually: 
// - &'a T where T: DllSafe<'lib> is always DllSafe, whether 'a: 'lib or 'lib: 'a

unsafe impl<'a, 'lib> DllSafe<'lib> for &'a T where T: DllSafe<'lib>, 'a: 'lib {
	type Target = &' lib T;
}

```

* issue: lifetime un-elision for T<'a>
	* fn(&self) -> T, with T<'a> is valid
	* must unelide to fn(&'a self) -> T<'a>, but nothing is known about T

* unspellable fn types:
```rust 
use std::cell::Cell;
use std::marker::PhantomData;

struct Invariant<'a>(Cell<&'a i32>);

impl<'a> Invariant<'a> {
    fn new(r: &'a i32) -> Invariant<'a> {
        Invariant(Cell::new(r))
    }
}

fn test<'a>(a: &Invariant<'a>, b: &Invariant<'a>) -> Invariant<'a>
{
    if a.0.get() > b.0.get() {
        Invariant(Cell::new(a.0.get()))
    } else {
        Invariant(Cell::new(b.0.get()))
    }
}


/*fn test<'a, 'b, 'min>(a: &Invariant<'a>, b: &Invariant<'b>) -> Invariant<'min>
where
    'a: 'min,
    'b: 'min,
{
    if a.0.get() > b.0.get() {
        Invariant(Cell::new(a.0.get()))
    } else {
        Invariant(Cell::new(b.0.get()))
    }
}*/
fn main()
{
let a = Invariant::new(&5);

{
let b = 3;
let b = Invariant::new(&b);
let c = test(&a, &b);
}

println!("{}", a.0.get());
}
```

#### Move to gfx-rs
* consider replacing the backend with gfx-rs
	* or actually test a gfx-rs backend?
	* not sure that's worth it



