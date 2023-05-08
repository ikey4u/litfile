#!/usr/bin/env python3
#!-*- coding:utf-8 -*-

import lit.formats
from lit.llvm import llvm_config
from lit.llvm.subst import ToolSubst
from pathlib import Path
import platform

config.name = "LITFILE TEST TOOL"
config.test_format = lit.formats.ShTest(True)
### __LITFILE_SUFFIX__
config.test_source_root = os.path.dirname(__file__)
config.excludes = []
### __LITFILE_SUBSTITUTIONS__
