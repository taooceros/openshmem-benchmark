use std::{
    iter::repeat_with,
    ops::{Deref, DerefMut},
};

use bon::bon;
use openshmem_benchmark::{
    osm_scope::{self},
    osm_vec::ShVec,
};

use crate::ops::Operation;

pub(crate) struct RangeBenchmarkData<'a> {
    pub src_working_set: WorkingSet<'a>,
    pub dst_working_set: WorkingSet<'a>,
}

#[bon]
impl<'a> RangeBenchmarkData<'a> {
    #[builder]
    pub fn setup_data(
        scope: &'a osm_scope::OsmScope,
        epoch_size: usize,
        data_size: usize,
        num_working_set: usize,
    ) -> RangeBenchmarkData<'a> {
        let mut source = Vec::with_capacity(num_working_set);
        let mut dest = Vec::with_capacity(epoch_size);

        for _ in 0..num_working_set {
            let mut data = repeat_with(|| ShVec::new(scope))
                .take(epoch_size)
                .collect::<Vec<_>>();

            data.iter_mut().for_each(|d| d.resize_with(data_size, || 0));

            source.push(Epoch::new(data));

            let mut data = repeat_with(|| ShVec::new(scope))
                .take(epoch_size)
                .collect::<Vec<_>>();

            data.iter_mut().for_each(|d| d.resize_with(data_size, || 0));

            dest.push(Epoch::new(data));
        }

        RangeBenchmarkData {
            src_working_set: WorkingSet::new(source),
            dst_working_set: WorkingSet::new(dest),
        }
    }
}

pub struct WorkingSet<'a> {
    pub epoches: Vec<Epoch<'a>>,
}

impl<'a> WorkingSet<'a> {
    pub fn new(epoches: Vec<Epoch<'a>>) -> Self {
        WorkingSet { epoches }
    }
}

impl<'a> Deref for WorkingSet<'a> {
    type Target = Vec<Epoch<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.epoches
    }
}

impl DerefMut for WorkingSet<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.epoches
    }
}

pub struct Epoch<'a> {
    pub data: Vec<ShVec<'a, u8>>,
}

impl<'a> Deref for Epoch<'a> {
    type Target = Vec<ShVec<'a, u8>>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a> DerefMut for Epoch<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<'a> Epoch<'a> {
    pub fn new(data: Vec<ShVec<'a, u8>>) -> Self {
        Epoch { data }
    }
}

impl RangeBenchmarkData<'_> {
    pub fn num_working_set(&self) -> usize {
        self.src_working_set.len()
    }

    pub fn epoch_size(&self) -> usize {
        self.src_working_set.epoch_size()
    }

    pub fn data_size(&self) -> usize {
        self.src_working_set.data_size()
    }
}

impl WorkingSet<'_> {
    pub fn len(&self) -> usize {
        self.epoches.len()
    }

    pub fn epoch_size(&self) -> usize {
        assert!(self.epoches.len() > 0);

        self.epoches[0].data.len()
    }

    pub fn data_size(&self) -> usize {
        assert!(self.epoches.len() > 0);
        assert!(self.epoches[0].data.len() > 0);

        self.epoches[0].data[0].len()
    }
}

impl Epoch<'_> {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn data_size(&self) -> usize {
        assert!(self.data.len() > 0);

        self.data[0].len()
    }
}
