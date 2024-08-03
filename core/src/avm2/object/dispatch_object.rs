//! Dispatch list object representation

use crate::avm2::activation::Activation;
use crate::avm2::events::DispatchList;
use crate::avm2::object::script_object::ScriptObjectData;
use crate::avm2::object::{Object, ObjectPtr, TObject};
use crate::avm2::value::Value;
use crate::avm2::Error;
use core::fmt;
use gc_arena::barrier::unlock;
use gc_arena::{lock::RefLock, Collect, Gc, GcWeak, Mutation};
use std::cell::{Ref, RefMut};

/// Internal representation of dispatch lists as generated by `EventDispatcher`.
///
/// This object is not intended to be constructed, subclassed, or otherwise
/// interacted with by user code. It exists solely to hold event handlers
/// attached to other objects. It's internal construction is subject to change.
/// Objects of this type are only accessed as private properties on
/// `EventDispatcher` instances.
///
/// `DispatchObject` exists primarily due to the generality of the class it
/// services. It has many subclasses, some of which may have different object
/// representations than `ScriptObject`. Furthermore, at least one
/// representation, `StageObject`, requires event dispatch to be able to access
/// handlers on parent objects. These requirements and a few other design goals
/// ruled out the following alternative scenarios:
///
/// 1. Adding event dispatch lists onto other associated data, such as
///    `DisplayObject`s. This would result in bare dispatchers not having a
///    place to store their data.
/// 2. Adding `DispatchList` to the `Value` enum. This would unnecessarily
///    complicate `Value` for an internal type, especially the comparison
///    logic.
/// 3. Making `DispatchObject` the default representation of all
///    `EventDispatcher` classes. This would require adding `DispatchList` to
///    other object representations that need to dispatch events, such as
///    `StageObject`.
#[derive(Clone, Collect, Copy)]
#[collect(no_drop)]
pub struct DispatchObject<'gc>(pub Gc<'gc, DispatchObjectData<'gc>>);

#[derive(Clone, Collect, Copy, Debug)]
#[collect(no_drop)]
pub struct DispatchObjectWeak<'gc>(pub GcWeak<'gc, DispatchObjectData<'gc>>);

impl fmt::Debug for DispatchObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DispatchObject")
            .field("ptr", &Gc::as_ptr(self.0))
            .finish()
    }
}

#[derive(Clone, Collect)]
#[collect(no_drop)]
#[repr(C, align(8))]
pub struct DispatchObjectData<'gc> {
    /// Base script object
    base: ScriptObjectData<'gc>,

    /// The dispatch list this object holds.
    dispatch: RefLock<DispatchList<'gc>>,
}

const _: () = assert!(std::mem::offset_of!(DispatchObjectData, base) == 0);
const _: () = assert!(
    std::mem::align_of::<DispatchObjectData>() == std::mem::align_of::<RefLock<ScriptObjectData>>()
);

impl<'gc> DispatchObject<'gc> {
    /// Construct an empty dispatch list.
    pub fn empty_list(activation: &mut Activation<'_, 'gc>) -> Object<'gc> {
        let base = ScriptObjectData::new(activation.avm2().classes().object);

        DispatchObject(Gc::new(
            activation.context.gc_context,
            DispatchObjectData {
                base,
                dispatch: RefLock::new(DispatchList::new()),
            },
        ))
        .into()
    }
}

impl<'gc> TObject<'gc> for DispatchObject<'gc> {
    fn gc_base(&self) -> Gc<'gc, ScriptObjectData<'gc>> {
        // SAFETY: Object data is repr(C), and a compile-time assert ensures
        // that the ScriptObjectData stays at offset 0 of the struct- so the
        // layouts are compatible

        unsafe { Gc::cast(self.0) }
    }

    fn as_ptr(&self) -> *const ObjectPtr {
        Gc::as_ptr(self.0) as *const ObjectPtr
    }

    fn construct(
        self,
        _activation: &mut Activation<'_, 'gc>,
        _args: &[Value<'gc>],
    ) -> Result<Object<'gc>, Error<'gc>> {
        Err("Cannot construct internal event dispatcher structures.".into())
    }

    fn value_of(&self, _mc: &Mutation<'gc>) -> Result<Value<'gc>, Error<'gc>> {
        Err("Cannot subclass internal event dispatcher structures.".into())
    }

    /// Unwrap this object as a list of event handlers.
    fn as_dispatch(&self) -> Option<Ref<DispatchList<'gc>>> {
        Some(self.0.dispatch.borrow())
    }

    /// Unwrap this object as a mutable list of event handlers.
    fn as_dispatch_mut(&self, mc: &Mutation<'gc>) -> Option<RefMut<DispatchList<'gc>>> {
        Some(unlock!(Gc::write(mc, self.0), DispatchObjectData, dispatch).borrow_mut())
    }
}
