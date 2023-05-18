use super::{
    dataflow::{DFGBuilder, DFGWrapper},
    handle::BuildHandle,
    BasicBlockID, BuildError, CfgID, Container, Dataflow, Wire,
};

use crate::{type_row, types::SimpleType};

use crate::ops::handle::NodeHandle;
use crate::ops::{BasicBlockOp, OpType};

use portgraph::NodeIndex;

use crate::{hugr::HugrMut, types::TypeRow, Hugr};

/// Builder for a [`crate::ops::controlflow::ControlFlowOp::CFG`] child control
/// flow graph
pub struct CFGBuilder<'f> {
    pub(super) base: &'f mut HugrMut,
    pub(super) cfg_node: NodeIndex,
    pub(super) inputs: Option<TypeRow>,
    pub(super) exit_node: NodeIndex,
    pub(super) n_out_wires: usize,
}

impl<'f> Container for CFGBuilder<'f> {
    type ContainerHandle = BuildHandle<CfgID>;

    #[inline]
    fn container_node(&self) -> NodeIndex {
        self.cfg_node
    }

    #[inline]
    fn base(&mut self) -> &mut HugrMut {
        self.base
    }

    #[inline]
    fn hugr(&self) -> &Hugr {
        self.base.hugr()
    }

    #[inline]
    fn finish(self) -> Self::ContainerHandle {
        (self.cfg_node, self.n_out_wires).into()
    }
}

impl<'f> CFGBuilder<'f> {
    /// Return a builder for a non-entry [`BasicBlockOp::Block`] child graph with `inputs`
    /// and `outputs` and the variants of the branching predicate Sum value
    /// specified by `predicate_variants`.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error adding the node.
    pub fn block_builder<'a: 'b, 'b>(
        &'a mut self,
        inputs: TypeRow,
        predicate_variants: Vec<TypeRow>,
        other_outputs: TypeRow,
    ) -> Result<BlockBuilder<'b>, BuildError> {
        let n_cases = predicate_variants.len();
        let op = OpType::BasicBlock(BasicBlockOp::Block {
            inputs: inputs.clone(),
            other_outputs: other_outputs.clone(),
            predicate_variants: predicate_variants.clone(),
        });
        let exit = self.exit_node;
        let block_n = self.base().add_op_before(exit, op)?;

        self.base().set_num_ports(block_n, 0, n_cases);

        // The node outputs a predicate before the data outputs of the block node
        let predicate_type = SimpleType::new_predicate(predicate_variants);
        let node_outputs: TypeRow = [&[predicate_type], other_outputs.as_ref()].concat().into();
        let db = DFGBuilder::create_with_io(self.base(), block_n, inputs, node_outputs)?;
        Ok(BlockBuilder::new(db))
    }

    /// Return a builder for a non-entry [`BasicBlockOp::Block`] child graph with `inputs`
    /// and `outputs` and a simple predicate type: a Sum of `n_cases` unit types.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error adding the node.
    pub fn simple_block_builder<'a: 'b, 'b>(
        &'a mut self,
        inputs: TypeRow,
        outputs: TypeRow,
        n_cases: usize,
    ) -> Result<BlockBuilder<'b>, BuildError> {
        self.block_builder(inputs, vec![type_row![]; n_cases], outputs)
    }

    /// Return a builder for the entry [`BasicBlockOp::Block`] child graph with `inputs`
    /// and `outputs` and the variants of the branching predicate Sum value
    /// specified by `predicate_variants`.
    ///
    /// # Errors
    ///
    /// This function will return an error if an entry block has already been built.
    pub fn entry_builder<'a: 'b, 'b>(
        &'a mut self,
        predicate_variants: Vec<TypeRow>,
        other_outputs: TypeRow,
    ) -> Result<BlockBuilder<'b>, BuildError> {
        let inputs = self
            .inputs
            .take()
            .ok_or(BuildError::EntryBuiltError(self.cfg_node))?;
        self.block_builder(inputs, predicate_variants, other_outputs)
    }

    /// Return a builder for the entry [`BasicBlockOp::Block`] child graph with `inputs`
    /// and `outputs` and a simple predicate type: a Sum of `n_cases` unit types.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error adding the node.
    pub fn simple_entry_builder<'a: 'b, 'b>(
        &'a mut self,
        outputs: TypeRow,
        n_cases: usize,
    ) -> Result<BlockBuilder<'b>, BuildError> {
        self.entry_builder(vec![type_row![]; n_cases], outputs)
    }

    /// Returns the exit block of this [`CFGBuilder`].
    pub fn exit_block(&self) -> BasicBlockID {
        self.exit_node.into()
    }

    /// Set the `branch` index `successor` block of `predecessor`.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an error connecting the blocks.
    pub fn branch<'a>(
        &mut self,
        predecessor: impl Into<&'a BasicBlockID>,
        branch: usize,
        successor: &BasicBlockID,
    ) -> Result<(), BuildError> {
        let predecessor: &BasicBlockID = predecessor.into();
        let from = predecessor.node();
        let to = successor.node();
        let base = &mut self.base;
        let hugr = base.hugr();
        let tin = hugr.num_inputs(to);
        let tout = hugr.num_outputs(to);

        base.set_num_ports(to, tin + 1, tout);
        Ok(base.connect(from, branch, to, tin)?)
    }
}

/// Builder for a [`BasicBlockOp::Block`] child graph.
pub type BlockBuilder<'b> = DFGWrapper<'b, BasicBlockID>;

impl<'b> BlockBuilder<'b> {
    /// Set the outputs of the block, with `branch_wire` being the value of the
    /// predicate.  `outputs` are the remaining outputs.
    pub fn set_outputs(
        &mut self,
        branch_wire: Wire,
        outputs: impl IntoIterator<Item = Wire>,
    ) -> Result<(), BuildError> {
        Dataflow::set_outputs(self, [branch_wire].into_iter().chain(outputs.into_iter()))
    }
    /// [Set outputs](BlockBuilder::set_outputs) and [finish](`BlockBuilder::finish`).
    pub fn finish_with_outputs(
        mut self,
        branch_wire: Wire,
        outputs: impl IntoIterator<Item = Wire>,
    ) -> Result<<BlockBuilder<'b> as Container>::ContainerHandle, BuildError>
    where
        Self: Sized,
    {
        self.set_outputs(branch_wire, outputs)?;
        Ok(self.finish())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        builder::{module_builder::ModuleBuilder, test::NAT},
        ops::ConstValue,
        type_row,
        types::Signature,
    };

    use super::*;
    #[test]
    fn basic_cfg() -> Result<(), BuildError> {
        let sum2_variants = vec![type_row![NAT], type_row![NAT]];

        let build_result = {
            let mut module_builder = ModuleBuilder::new();
            let main =
                module_builder.declare("main", Signature::new_df(vec![NAT], type_row![NAT]))?;
            let s1 = module_builder.constant(ConstValue::simple_unary_predicate())?;
            let _f_id = {
                let mut func_builder = module_builder.define_function(&main)?;
                let [int] = func_builder.input_wires_arr();

                let cfg_id = {
                    let mut cfg_builder =
                        func_builder.cfg_builder(vec![(NAT, int)], type_row![NAT])?;
                    let mut entry_b =
                        cfg_builder.entry_builder(sum2_variants.clone(), type_row![])?;

                    let entry = {
                        let [inw] = entry_b.input_wires_arr();

                        let sum = entry_b.make_predicate(1, sum2_variants, [inw])?;
                        entry_b.finish_with_outputs(sum, [])?
                    };
                    let mut middle_b =
                        cfg_builder.simple_block_builder(type_row![NAT], type_row![NAT], 1)?;

                    let middle = {
                        let c = middle_b.load_const(&s1)?;
                        let [inw] = middle_b.input_wires_arr();
                        middle_b.finish_with_outputs(c, [inw])?
                    };

                    let exit = cfg_builder.exit_block();

                    cfg_builder.branch(&entry, 0, &middle)?;
                    cfg_builder.branch(&middle, 0, &exit)?;
                    cfg_builder.branch(&entry, 1, &exit)?;

                    cfg_builder.finish()
                };

                func_builder.finish_with_outputs(cfg_id.outputs())?
            };
            module_builder.finish()
        };

        assert_eq!(build_result.err(), None);

        Ok(())
    }
}