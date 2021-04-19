if exists("b:current_syntax")
    finish
endif

let b:current_syntax = "bear"

syn match bear_number '@'
syn match bear_number '\d\+'
syn match bear_number '[-+]\d\+'
syn match bear_number '0x[0-9a-fA-F]\+'

syn match bear_label '\$'
syn match bear_label '\$>'
syn match bear_label '<\$'
syn match bear_label '^=*:[a-zA-Z_]\+[:a-zA-Z_0-9]*'
syn match bear_label_ref '&[a-zA-Z_]\+[:a-zA-Z_0-9]*'

syn match bear_ident "[a-zA-Z_]\+[a-zA-Z_0-9\-]*"
syn match bear_definition_ref "![a-zA-Z_]\+[a-zA-Z_0-9\-]*"

syn region bear_list start='\[' end='\]' contains=bear_kw,bear_comment,bear_definition_ref,bear_string_lit,bear_number,bear_label_ref
syn region bear_directive start="^#" end=';' contains=bear_list

syn region bear_string_lit start='"' end='"'

syn match bear_comment "--.*$"
syn match bear_kw /[a-z]\+[a-z.0-9]*/
syn match bear_quoted /`[a-z]\+[a-z.0-9]*/

hi def link bear_quoted         Quoted
hi def link bear_list           Macro
hi def link bear_string_lit     String
hi def link bear_directive      PreProc
hi def link bear_definition_ref PreProc
hi def link bear_number         Constant
hi def link bear_label          Label
hi def link bear_label_ref      Special
hi def link bear_ident          Identifier
hi def link bear_comment        Comment
hi def link bear_kw             Keyword


