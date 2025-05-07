#pragma once

#include "eggmock.h"

#include <cstdint>

extern "C"
{
  struct fcdram_compiler_statistics
  {
    uint64_t egraph_classes;
    uint64_t egraph_nodes;
    uint64_t egraph_size;

    uint64_t instruction_count;

    uint64_t t_runner;
    uint64_t t_extractor;
    uint64_t t_compiler;
  };

  struct fcdram_compiler_settings
  {
    bool print_program;
    bool verbose;
  };

  eggmock::mig_receiver<eggmock::mig_rewrite> fcdram_rewriter( fcdram_compiler_settings settings );
  eggmock::mig_receiver<fcdram_compiler_statistics> fcdram_compile( fcdram_compiler_settings settings );
}
