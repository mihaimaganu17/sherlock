; This file implements random trampolines between 16-bit, 32-bit, and 64-bit modes. We don't do
; this in inline assembly as we don't have the ability to change the bitness of the instructions
; dynamically
[bits 32]
