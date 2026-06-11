;; a b c -- c a b ;;
(@any @any @any) { rot rot } into unrot

;; a b -- b ;;
(@any @any) { swap drop } into nip

;; a b -- b a b ;;
(@any @any) { dup unrot } into tuck

;; a b -- a a b;;
(@any @any) { over swap } into cover

;; a b -- a b a b ;;
(@any @any) { over over } into 2dup

;; a b -- ;;
(@any @any) { drop drop } into 2drop

;; a -- [a] ;;
(@any) { push 1 pack } into quote

(@any) {
	int? if { } else {
		push "Could not cast into int" println
	}
} into as-int

(@any) {
	char? if { } else {
		push "Could not cast into char" println
	}
} into as-char

(@any) {
	string? if { } else {
		push "Could not cast into string" println
	}
} into as-string


(@int) {
	bool? if { } else {
		push "Could not cast into bool" println
	}
	not
} into null?

[
	([..@any]) {
		bool? if { } else {
			push "Could not cast into bool" println
		}
		not
	}
	(@string) {
		bool? if { } else {
			push "Could not cast into bool" println
		}
		not
	}
	(..) {
		push "Could not check if it is empty" println
	}
] into empty?

[
	(@string @char){
		find? if { } else {
			push "Could not find pattern in string" println
		}
	}
	(@string @string){
		find? if { } else {
			push "Could not find pattern in string" println
		}
	}
] into find

;; a fn -- fn(x) a ;;
(@any @function) {
	swap into tmp
	eval
	take tmp
} into dip

;; a b fn -- fn(x) a b ;;
(@any @any @function) {
	swap into tmp1
	swap into tmp2
	eval
	take tmp2
	take tmp1
} into 2dip

;; a fn -- a fn(a);;
(@any @function) {
	cover
	eval
} into keep

;; a fn1 fn2 -- fn1(a) fn2(a);;
(@any @function @function) {
	push 3 pick
	swap
	eval
	swap dip
} into bi

;; a fn1 fn2 fn2 -- fn1(a) fn2(a) fn3(a);;
(@any @function @function @function) {
	push 4 pick
	swap
	eval
	push 4 pick
	rot eval
	push 4 roll
	push 4 roll
	eval
	unrot swap
} into tri

[
	(@int @int) {
		2dup gt
		if { swap } else { }
		drop
	}

] into min

[
	(@string @char) { concat }
	(@string @string) { concat }
	([..@any] @any) { quote concat }
] into append

(@string @int) {
	push 1
	swap
	substr
	as-char
} into getchar

[
	(@string @char) {
		over swap find
		2dup
		push 0 substr
		rot rot push 1 add swap
		dup len push 3 sub
		rot substr
	}
	(@string @string) {
		over swap find
		2dup
		push 0 substr
		rot rot push 1 add swap
		dup len push 3 sub
		rot substr
	}
] into split

[
	;; map a list ;;
	([..@any] @function) {
		over len into l
		swap explode
		l push 1 add roll
		l swap
		map
		take l pack
	}
	;; map n elements of the stack ;;
	(..@any @int @function) {
		push 0 swap
			(..@any @int @int @function @function) {
				push 4 pick
				push 4 pick
				gt if {
					push 4 pick
						push 4 add
						roll
						over
						eval
						push 5 roll
						push 5 roll
						push 1 add
						push 5 roll
						push 5 roll
						over
						eval
					} else {
						drop drop drop drop
					}
			}
		swap over eval
	}
]into map

;;
[
    ( [..@any] @any @any ) {
        swap
        push 1 pack
        swap

        [
            ( [] [..@any] @any @any ) {
                drop
                drop
                swap
                drop
            }

            ( [ @any | [..@any] ] [..@any] @any @any ) {
                push 2 pick
                push 0 at

                push 4 pick

                push 3 pick

                match
                push 4 roll
                drop
                push 3 roll
                swap
                cons
                rot rot

                dup match
            }
        ]

        dup match
    }

    ( .. ) {
        push "Error: scan arguments mismatch!" println
    }
] into scan
;;

[
	( [@int | [..@int] ] ) {
		uncon swap
		rot rot
		over over
		lt
	}
] into sort

([..@int]){
	uncon swap
	( @function @int [..@int] ){
		dup len push 0
		gt if {
			uncon rot
			add
			swap
			push 3 pick
			eval
		} else {
			drop
			swap drop
		}
	}
	rot rot
	push 3 pick eval
} into sum-list

(@string @string) {
    into sep

    (@string) {
        push sep dup into sep
		split
        if {
			swap
            rot dup
			rot swap
			eval
        } else {
            swap drop
        }
    }

    swap over eval
	push sep drop
} into splitall

(..@any @string) {
	push "%"
	split
	if {
		print
		swap
		print
		call printf
	} else {
		print
	}
} into printf

(@any) {
	push 1
	swap
	string?
	if {
		push "\n"
		concat
		sys-write
	} else {
		drop
		push "Could not print element" println
	}
} into println

(@any) {
	push 1
	swap
	sys-write
} into print

(@string @int) {
	dup
	push 0
	eq
	not
	if {
		over print
		push 1
		sub
		call repeat
	} else {
		push "\n" print
		drop
	}
} into repeat

;; recebe o fd como argumento;;
(@int) {
	push 1
	sys-read
} into read-char
