
extern crate gfx_hal as gfx;

use gfx::Backend;
use gfx::command::RenderPassInlineEncoder;
use gfx::queue::Supports;
use std::any::Any;
use std::ops::Range;

// TODO:
pub trait Device<B: Backend> {
    fn create_frame_graph(
        &self,
    ) -> FrameGraph<B>;
}

impl<B> Device<B> for B::Device
where
    B: Backend,
{
    fn create_frame_graph(
        &self,
    ) -> FrameGraph<B> {
        unimplemented!()
    }
}

/// Execution dependency between two frame graph nodes (A -> B).
/// A will be executed before B.
pub type Dependency = Range<PassId>;

/// Frame relative to the current frame graph execution.
/// Required to address resources of previouse frames (e.g for TAA).
pub type Frame = isize;

#[derive(Debug, Copy, Clone)]
pub struct BufferRef(usize, Frame);
#[derive(Debug, Copy, Clone)]
pub struct BufferViewRef(usize, Frame);
#[derive(Debug, Copy, Clone)]
pub struct ImageRef(usize, Frame);
#[derive(Debug, Copy, Clone)]
pub struct ImageViewRef(usize, Frame);

/// Id for a pass when building the frame graph.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PassId(usize);

pub struct FrameGraph<B: Backend> {
    _temp: std::marker::PhantomData<B>,
}

/// Internal storage of pass specific resource information.
struct PassDataStorage {
    data: Vec<Box<Any>>,
}

impl PassDataStorage {
    fn fetch<'a, T: Copy>(&'a self, index: usize) -> T {
        let data = &self.data[index];
        unsafe { *(data as *const Any as *const T) }
    }
}

/// Inline graphics pass, roughly equals an inline subpass.
pub type GraphicsPass<B, R> = fn(RenderPassInlineEncoder<B>, R);

/// Id for a registerd queue family. Frame graph nodes are associated with a certain queue.
/// Use this for identification when recording and submitting a frame graph to one or multiple queues.
#[derive(Debug, Copy, Clone)]
pub struct FamilyId<C>(usize, std::marker::PhantomData<C>);

/// Number of queues.
pub type QueueCount = usize;
/// Queue index in registered family.
pub type QueueId = usize;

/// Logical resource table.
/// Logical resources can alias memory with the same physical resources depending on their lifetime.
/// The frame graph builder will internally allocate the minimum number of physical resources required
/// for holding all logical resources.
struct LogicalResources {
    buffers: Vec<()>,
}

impl LogicalResources {
    fn new() -> Self {
        LogicalResources {
            buffers: Vec::new(),
        }
    }

    fn add_buffer(&mut self, name: &str, frame: Frame) -> BufferRef {
        // TODO
        let id = self.buffers.len();
        self.buffers.push(());
        BufferRef(id, frame)
    }
}

/// Builder for a logical frame graph.
pub struct FrameGraphBuilder<'b, B: Backend> {
    /// Pass specific resource data.
    data_storage: PassDataStorage,
    /// Logical resources.
    logical_resources: LogicalResources,
    /// Registered families with their associated queue counts.
    families: Vec<QueueCount>,
    /// Current passes with their exeuction dependencies.
    passes: Vec<Box<FnMut(RenderPassInlineEncoder<B>, &'b PassDataStorage, &'b FrameGraph<B>) -> () + 'b>>,
    dependencies: Vec<Dependency>,
}

impl<'c, B: Backend> FrameGraphBuilder<'c, B> {
    pub fn new() -> Self {
        FrameGraphBuilder {
            data_storage: PassDataStorage { data: Vec::new() },
            logical_resources: LogicalResources::new(),
            families: Vec::new(),
            passes: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn register_queue_family<C: gfx::Capability>(
        &mut self,
        family: &B::QueueFamily,
        num_queues: usize,
    ) -> FamilyId<C> {
        let id = FamilyId(self.families.len(), std::marker::PhantomData);
        self.families.push(num_queues);
        id
    }

    pub fn create_buffer(&mut self, name: &str, frame: Frame) -> BufferRef {
        self.logical_resources.add_buffer(name, frame)
    }

    pub fn add_graphic<C, D, R>(
        &mut self,
        queue: (FamilyId<C>, QueueId),
        setup: R,
        pass: GraphicsPass<B, R::Resources>,
    ) -> PassId
    where
        C: Supports<gfx::Graphics>,
        R: PassResources<'c, B>,
    {
        let id = self.data_storage.data.len();
        self.data_storage.data.push(Box::new(setup));

        let pass_id = self.passes.len();
        self.passes.push(Box::new(move |encoder: RenderPassInlineEncoder<B>, data: &'c PassDataStorage, res: &'c FrameGraph<B>| {
            let a = data.fetch(id);
            let d = <R as PassResources<'c, B>>::acquire(a, res);
            pass(encoder, d);
        }));
        PassId(pass_id)
    }

    pub fn add_dependency(&mut self, dep: Dependency) {
        assert!(dep.start < dep.end);
        self.dependencies.push(dep);
    }
}


//# Tests

#[derive(Debug, Copy, Clone)]
pub struct OceanResourcesVirtual {
    pub spectrum: BufferRef,
}

pub struct OceanResources {
    // pub spectrum: &'a B::Buffer,
}

pub trait PassResources<'a, B: Backend> : Copy + 'static {
    type Resources: 'a;
    fn acquire(&self, frame_graph: &'a FrameGraph<B>) -> Self::Resources;
}

impl<'a, B: Backend> PassResources<'a, B> for OceanResourcesVirtual {
    type Resources = OceanResources;
    fn acquire(&self, frame_graph: &'a FrameGraph<B>) -> OceanResources {
        OceanResources { }
    }
}

fn render_ocean<B: Backend>(encoder: RenderPassInlineEncoder<B>, resources: OceanResources) {
    println!("hello");
}

#[test]
fn try_build() {
    let mut builder = FrameGraphBuilder::new();
    builder.add(
        OceanResourcesVirtual {
            spectrum: BufferRef(0),
        },
        render_ocean,
    );

    let frame_graph = FrameGraph { _temp: std::marker::PhantomData };
    for pass in &builder.passes {
        pass(&builder.data_storage, &frame_graph);
    }
}
