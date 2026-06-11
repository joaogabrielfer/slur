fun double {
	len lt 1 if {
		ret
	}
	push 2
	mul
	dup pop
	push " " pop
}

fun is_eight {
	len lt 1 if {
		ret
	}
	dup
	eq 8 not if{
		ret
	}
	drop
	push "é oito" pop
	push "\n" pop
}

push 1

call double
call is_eight
call double
call is_eight
call double
call is_eight
call double
call is_eight
call double
call is_eight
