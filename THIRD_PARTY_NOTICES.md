# Third-Party Notices

Yune's original code is licensed under the MIT License. Some checked-in
schemas, dictionaries, fixtures, generated data, and provenance materials are
from or derived from upstream projects and remain under their own terms.

Known third-party material in this repository includes:

- TypeDuck-Web-derived browser harness and TypeDuck Cantonese assets used by
  `apps/yune-web/`; see `apps/yune-web/public-demo/PROVENANCE.md`. The upstream
  TypeDuck-Web repository is BSD-3-Clause:
  https://github.com/TypeDuck-HK/TypeDuck-Web/blob/main/LICENSE

  ```text
  BSD 3-Clause License

  Copyright (c) 2024, TypeDuck Team

  Redistribution and use in source and binary forms, with or without
  modification, are permitted provided that the following conditions are met:

  1. Redistributions of source code must retain the above copyright notice, this
     list of conditions and the following disclaimer.

  2. Redistributions in binary form must reproduce the above copyright notice,
     this list of conditions and the following disclaimer in the documentation
     and/or other materials provided with the distribution.

  3. Neither the name of the copyright holder nor the names of its
     contributors may be used to endorse or promote products derived from
     this software without specific prior written permission.

  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
  AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
  IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
  FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
  DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
  CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
  OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
  OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
  ```
- Rime Luna Pinyin schema/dictionary assets under `apps/yune-web/public/schema/`
  and oracle fixtures under `crates/yune-core/tests/fixtures/`; upstream
  `rime/rime-luna-pinyin` is LGPL-3.0:
  https://github.com/rime/rime-luna-pinyin/blob/master/LICENSE
- Rime Cantonese/Jyutping-derived schema and dictionary assets under
  `apps/yune-web/public/schema/`; upstream `rime/rime-cantonese` documents
  CC-BY-4.0 for the main work and ODbL-1.0 for `jyut6ping3.maps`:
  https://github.com/rime/rime-cantonese
- Cangjie5 and Cangjie3-Plus-derived schema/dictionary assets under
  `apps/yune-web/public/schema/`; the upstream repositories document MIT
  licensing:
  https://github.com/Jackchows/Cangjie5
  https://github.com/Arthurmcarthur/Cangjie3-Plus
- OpenCC dictionary data under `crates/yune-core/src/opencc/data/` and generated
  browser OpenCC assets under `apps/yune-web/public/schema/opencc/`; upstream
  OpenCC documents Apache-2.0 licensing:
  https://github.com/BYVoid/OpenCC/blob/master/LICENSE

This notice is not a substitute for the upstream licenses. Keep source headers,
per-directory provenance files, and generated-asset manifests intact when
redistributing the repository or public demo bundle.
