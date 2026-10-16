# Third-party notices

## Microsoft Windows-driver-samples (MS-PL)

Source: <https://github.com/microsoft/Windows-driver-samples>, folder
`audio/simpleaudiosample`. Copyright (c) Microsoft Corporation. Licensed
under the Microsoft Public License; full text in
[LICENSES/MS-PL.txt](LICENSES/MS-PL.txt).

These files are derived from that sample, keep Microsoft's copyright notice,
and are distributed under MS-PL:

- `AudioProgramCable.inx`
- `src/adapter.cpp`
- `src/common.cpp`, `src/common.h`
- `src/cabletables.h`, `src/endpoints.h`, `src/definitions.h`
- `src/kshelper.cpp`, `src/kshelper.h`
- `src/mintopo.cpp`, `src/mintopo.h`
- `src/minwavert.cpp`, `src/minwavert.h`
- `src/minwavertstream.cpp`, `src/minwavertstream.h`
- `src/newdelete.cpp`, `src/newdelete.h`

Under MS-PL, a compiled `AudioProgramCable.sys` may be distributed under any
licence that complies with MS-PL; source distributions of these files must
keep the MS-PL notice.

## Sonicanvas Windows driver (MIT), smoke test only

Source: <https://github.com/open-string/sonicanvas-windows-driver>, commit
`d38f16a0fa848a7b85563b3e5fb65b82dd7721db`. Copyright (c) 2026 Yihui Xiong.
MIT License.

No Sonicanvas code is included in this repository.
`smoke-test/sonicanvas/Build-SonicanvasPackage.ps1` downloads it at build time
to produce a separate test package, which includes Sonicanvas's own
`LICENSE.txt`.

## Microsoft Windows SDK and WDK

The build downloads the Windows SDK and WDK NuGet packages (10.0.28000.2526)
under Microsoft's licence terms. They are build tools only and are not
redistributed in the driver package.
