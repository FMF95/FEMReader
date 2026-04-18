use crate::assembler::ModelAssembler;
use crate::cache::IncrementalCache;
use crate::dependency::DependencyResolver;
use crate::model::ModelContent;
use crate::scanner::FileSystemScanner;
use crate::serializer::Serializer;
use std::io;
use std::path::Path;

pub struct ReaderPipeline<S, D, A, Z>
where
    S: FileSystemScanner,
    D: DependencyResolver,
    A: ModelAssembler,
    Z: Serializer,
{
    pub scanner: S,
    pub dependency_resolver: D,
    pub model_assembler: A,
    pub serializer: Z,
}

impl<S, D, A, Z> ReaderPipeline<S, D, A, Z>
where
    S: FileSystemScanner,
    D: DependencyResolver,
    A: ModelAssembler,
    Z: Serializer,
{
    pub fn run(
        &self,
        root_directory: &Path,
        cache: &mut IncrementalCache,
    ) -> io::Result<ModelContent> {
        let scan_result = self.scanner.scan(root_directory, cache)?;
        let graph = self.dependency_resolver.resolve(&scan_result);
        Ok(self.model_assembler.assemble(&scan_result, &graph))
    }

    pub fn run_and_serialize(
        &self,
        root_directory: &Path,
        cache: &mut IncrementalCache,
        pretty: bool,
    ) -> io::Result<String> {
        let model = self.run(root_directory, cache)?;
        Ok(self.serializer.serialize(&model, pretty))
    }
}
