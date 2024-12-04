#!/bin/bash
if cargo test --lib -q "$@"
then
	play -q ../../sounds/success.ogg
else 
	play -q ../../sounds/failed.ogg
fi
