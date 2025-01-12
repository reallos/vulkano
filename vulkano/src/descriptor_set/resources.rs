// Copyright (c) 2017 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use crate::buffer::BufferViewAbstract;
use crate::descriptor_set::layout::{DescriptorSetLayout, DescriptorType};
use crate::descriptor_set::sys::{DescriptorWrite, DescriptorWriteElements};
use crate::descriptor_set::BufferAccess;
use crate::image::ImageViewAbstract;
use crate::sampler::Sampler;
use fnv::FnvHashMap;
use smallvec::{smallvec, SmallVec};
use std::sync::Arc;

/// The resources that are bound to a descriptor set.
#[derive(Clone)]
pub struct DescriptorSetResources {
    descriptors: FnvHashMap<u32, DescriptorBindingResources>,
}

impl DescriptorSetResources {
    /// Creates a new `DescriptorSetResources` matching the provided descriptor set layout, and
    /// all descriptors set to `None`.
    pub fn new(layout: &DescriptorSetLayout, variable_descriptor_count: u32) -> Self {
        assert!(variable_descriptor_count <= layout.variable_descriptor_count());

        let descriptors = layout
            .desc()
            .bindings()
            .iter()
            .enumerate()
            .filter_map(|(b, d)| d.as_ref().map(|d| (b as u32, d)))
            .map(|(binding_num, binding_desc)| {
                let count = if binding_desc.variable_count {
                    variable_descriptor_count
                } else {
                    binding_desc.descriptor_count
                } as usize;

                let binding_resources = match binding_desc.ty {
                    DescriptorType::UniformBuffer
                    | DescriptorType::StorageBuffer
                    | DescriptorType::UniformBufferDynamic
                    | DescriptorType::StorageBufferDynamic => {
                        DescriptorBindingResources::Buffer(smallvec![None; count])
                    }
                    DescriptorType::UniformTexelBuffer | DescriptorType::StorageTexelBuffer => {
                        DescriptorBindingResources::BufferView(smallvec![None; count])
                    }
                    DescriptorType::SampledImage
                    | DescriptorType::StorageImage
                    | DescriptorType::InputAttachment => {
                        DescriptorBindingResources::ImageView(smallvec![None; count])
                    }
                    DescriptorType::CombinedImageSampler => {
                        if binding_desc.immutable_samplers.is_empty() {
                            DescriptorBindingResources::ImageViewSampler(smallvec![None; count])
                        } else {
                            DescriptorBindingResources::ImageView(smallvec![None; count])
                        }
                    }
                    DescriptorType::Sampler => {
                        if binding_desc.immutable_samplers.is_empty() {
                            DescriptorBindingResources::Sampler(smallvec![None; count])
                        } else {
                            DescriptorBindingResources::None
                        }
                    }
                };
                (binding_num, binding_resources)
            })
            .collect();

        Self { descriptors }
    }

    /// Applies descriptor writes to the resources.
    ///
    /// # Panics
    ///
    /// - Panics if the binding number of a write does not exist in the resources.
    /// - See also [`DescriptorBindingResources::update`].
    pub fn update<'a>(&mut self, writes: impl IntoIterator<Item = &'a DescriptorWrite>) {
        for write in writes {
            self.descriptors
                .get_mut(&write.binding_num)
                .expect("descriptor write has invalid binding number")
                .update(write)
        }
    }

    /// Returns a reference to the bound resources for `binding`. Returns `None` if the binding
    /// doesn't exist.
    #[inline]
    pub fn binding(&self, binding: u32) -> Option<&DescriptorBindingResources> {
        self.descriptors.get(&binding)
    }
}

/// The resources that are bound to a single descriptor set binding.
#[derive(Clone)]
pub enum DescriptorBindingResources {
    None,
    Buffer(Elements<Arc<dyn BufferAccess>>),
    BufferView(Elements<Arc<dyn BufferViewAbstract>>),
    ImageView(Elements<Arc<dyn ImageViewAbstract>>),
    ImageViewSampler(Elements<(Arc<dyn ImageViewAbstract>, Arc<Sampler>)>),
    Sampler(Elements<Arc<Sampler>>),
}

type Elements<T> = SmallVec<[Option<T>; 1]>;

impl DescriptorBindingResources {
    /// Applies a descriptor write to the resources.
    ///
    /// # Panics
    ///
    /// - Panics if the resource types do not match.
    /// - Panics if the write goes out of bounds.
    pub fn update(&mut self, write: &DescriptorWrite) {
        fn write_resources<T: Clone>(first: usize, resources: &mut [Option<T>], elements: &[T]) {
            resources
                .get_mut(first..first + elements.len())
                .expect("descriptor write for binding out of bounds")
                .iter_mut()
                .zip(elements)
                .for_each(|(resource, element)| {
                    *resource = Some(element.clone());
                });
        }

        let first = write.first_array_element() as usize;

        match (self, write.elements()) {
            (
                DescriptorBindingResources::Buffer(resources),
                DescriptorWriteElements::Buffer(elements),
            ) => write_resources(first, resources, elements),
            (
                DescriptorBindingResources::BufferView(resources),
                DescriptorWriteElements::BufferView(elements),
            ) => write_resources(first, resources, elements),
            (
                DescriptorBindingResources::ImageView(resources),
                DescriptorWriteElements::ImageView(elements),
            ) => write_resources(first, resources, elements),
            (
                DescriptorBindingResources::ImageViewSampler(resources),
                DescriptorWriteElements::ImageViewSampler(elements),
            ) => write_resources(first, resources, elements),
            (
                DescriptorBindingResources::Sampler(resources),
                DescriptorWriteElements::Sampler(elements),
            ) => write_resources(first, resources, elements),
            _ => panic!(
                "descriptor write for binding {} has wrong resource type",
                write.binding_num,
            ),
        }
    }
}
