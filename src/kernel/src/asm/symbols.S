.section .rodata

.global TEXT_START
TEXT_START: .dword _text_start

.global TEXT_END
TEXT_END: .dword _text_end

.global RODATA_START
RODATA_START: .dword _rodata_start

.global RODATA_END
RODATA_END: .dword _rodata_end

.global DATA_START
DATA_START: .dword _data_start

.global DATA_END
DATA_END: .dword _stack_end

.global HEAP_START
HEAP_START: .dword _heap_start

.global HEAP_SIZE
HEAP_SIZE: .dword _heap_size

.global EH_FRAME_START
EH_FRAME_START: .dword _eh_frame_start

.global EH_FRAME_SIZE
EH_FRAME_SIZE: .dword _eh_frame_size
