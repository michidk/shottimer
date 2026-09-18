# Third-party notices

This project depends on third-party Rust crates. Their license metadata and
license files are distributed with their source packages. The notices below
cover material embedded directly in the firmware image or specifically adapted
by this project.

## U8g2 fonts

The firmware embeds `u8g2_font_helvR18_tr` and
`u8g2_font_logisoso58_tf` through the `u8g2-fonts` crate. The crate is
licensed under MIT OR Apache-2.0; the fonts have separate terms documented by
the [U8g2 project](https://github.com/olikraus/u8g2/blob/master/LICENSE).

### Helvetica R18

The Helvetica bitmap font is derived from the X11 `HELVR18.BDF` font and is
distributed under these terms:

Copyright 1984-1989, 1994 Adobe Systems Incorporated.
Copyright 1988, 1994 Digital Equipment Corporation.

Adobe is a trademark of Adobe Systems Incorporated which may be registered in
certain jurisdictions. Permission to use these trademarks is hereby granted
only in association with the images described in this file.

Permission to use, copy, modify, distribute and sell this software and its
documentation for any purpose and without fee is hereby granted, provided that
the above copyright notices appear in all copies and that both those copyright
notices and this permission notice appear in supporting documentation, and that
the names of Adobe Systems and Digital Equipment Corporation not be used in
advertising or publicity pertaining to distribution of the software without
specific, written prior permission. Adobe Systems and Digital Equipment
Corporation make no representations about the suitability of this software for
any purpose. It is provided "as is" without express or implied warranty.

### Logisoso

U8g2 identifies Logisoso as licensed under the SIL Open Font License. The font
copyright statement is:

> Created by Mathieu Gabiot with FontForge 2.0
> (http://fontforge.sf.net) - Brussels - 2009

Copyright (C) 2011 Mathieu-g (Mathieu Gabiot).

SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007

PREAMBLE

The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font creation
efforts of academic and linguistic communities, and to provide a free and open
framework in which fonts may be shared and improved in partnership with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The fonts,
including any derivative works, can be bundled, embedded, redistributed and/or
sold with any software provided that any reserved names are not used by
derivative works. The fonts and derivatives, however, cannot be released under
any other type of license. The requirement for fonts to remain under this
license does not apply to any document created using the fonts or their
derivatives.

DEFINITIONS

"Font Software" refers to the set of files released by the Copyright Holder(s)
under this license and clearly marked as such. This may include source files,
build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the copyright
statement(s).

"Original Version" refers to the collection of Font Software components as
distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to, deleting, or
substituting -- in part or in whole -- any of the components of the Original
Version, by changing formats or by porting the Font Software to a new
environment.

"Author" refers to any designer, engineer, programmer, technical writer or
other person who contributed to the Font Software.

PERMISSION & CONDITIONS

Permission is hereby granted, free of charge, to any person obtaining a copy of
the Font Software, to use, study, copy, merge, embed, modify, redistribute, and
sell modified and unmodified copies of the Font Software, subject to the
following conditions:

1) Neither the Font Software nor any of its individual components, in Original
or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy contains
the above copyright notice and this license. These can be included either as
stand-alone text files, human-readable headers or in the appropriate
machine-readable metadata fields within text or binary files as long as those
fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font Name(s)
unless explicit written permission is granted by the corresponding Copyright
Holder. This restriction only applies to the primary font name as presented to
the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font Software
shall not be used to promote, endorse or advertise any Modified Version, except
to acknowledge the contribution(s) of the Copyright Holder(s) and the Author(s)
or with their explicit written permission.

5) The Font Software, modified or unmodified, in part or in whole, must be
distributed entirely under this license, and must not be distributed under any
other license. The requirement for fonts to remain under this license does not
apply to any document created using the Font Software.

TERMINATION

This license becomes null and void if any of the above conditions are not met.

DISCLAIMER

THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF COPYRIGHT, PATENT,
TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE COPYRIGHT HOLDER BE LIABLE FOR
ANY CLAIM, DAMAGES OR OTHER LIABILITY, INCLUDING ANY GENERAL, SPECIAL, INDIRECT,
INCIDENTAL, OR CONSEQUENTIAL DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR
OTHERWISE, ARISING FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR
FROM OTHER DEALINGS IN THE FONT SOFTWARE.

## Design inspiration

The shot-detection behavior, timing defaults, and timer-screen concept were
inspired by
[`lspr98/profitec-go-waterlevel-shottimer`](https://github.com/lspr98/profitec-go-waterlevel-shottimer).
No source code from that repository is included here. At the time this notice
was written, the upstream repository did not declare a software license. This
notice is an acknowledgement and does not place the upstream project under this
project's MIT OR Apache-2.0 license.
