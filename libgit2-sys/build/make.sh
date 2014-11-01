#!/bin/sh

if [ `uname` = "FreeBSD" ]; then
	gmake $*
else
	make $*
fi
