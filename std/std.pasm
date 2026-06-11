;; a b c -- c a b ;;
(@any @any @any) -> (@any @any @any) { rot rot } into unrot

;; a b -- b ;;
(@any @any) -> (@any) { swap drop } into nip

;; a b -- b a b ;;
(@any @any) -> (@any @any @any) { dup unrot } into tuck

;; a b -- a a b;;
(@any @any) -> (@any @any @any) { over swap } into cover

;; a b -- a b a b ;;
(@any @any) -> (@any @any @any @any) { over over } into 2dup

;; a b -- ;;
(@any @any) -> () { drop drop } into 2drop

;; a -- [a] ;;
(@any) -> ([..@any]) { push 1 pack } into quote

(@any) -> (@int) {
	int? if { } else {
		push "Could not cast into int" println
	}
} into as-int

(@any) -> (@char) {
	char? if { } else {
		push "Could not cast into char" println
	}
} into as-char

(@any) -> (@string) {
	string? if { } else {
		push "Could not cast into string" println
	}
} into as-string


(@int) -> (@bool) {
	bool? if { } else {
		push "Could not cast into bool" println
	}
	not
} into null?

[
	([..@any]) -> (@bool) {
		bool? if { } else {
			push "Could not cast into bool" println
		}
		not
	}
	(@string) -> (@bool) {
		bool? if { } else {
			push "Could not cast into bool" println
		}
		not
	}
	(..) -> () {
		push "Could not check if it is empty" println
	}
] into empty?

[
	(@string @char) -> (@int) {
		find? if { } else {
			push "Could not find pattern in string" println
		}
	}
	(@string @string) -> (@int) {
		find? if { } else {
			push "Could not find pattern in string" println
		}
	}
] into find

;; a fn -- fn(x) a ;;
(@any @function) -> (..@any) {
	swap into tmp
	eval
	take tmp
} into dip

;; a b fn -- fn(x) a b ;;
(@any @any @function) -> (..@any) {
	swap into tmp1
	swap into tmp2
	eval
	take tmp2
	take tmp1
} into 2dip

;; a fn -- a fn(a);;
(@any @function) -> (..@any) {
	cover
	eval
} into keep

;; a fn1 fn2 -- fn1(a) fn2(a);;
(@any @function @function) -> (..@any) {
	push 3 pick
	swap
	eval
	swap dip
} into bi

;; a fn1 fn2 fn2 -- fn1(a) fn2(a) fn3(a);;
(@any @function @function @function) -> (..@any) {
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
	(@int @int) -> (@int) {
		2dup gt
		if { swap } else { }
		drop
	}

] into min

[
	(@string @char) -> (@string) { concat }
	(@string @string) -> (@string) { concat }
	([..@any] @any) -> ([..@any]) { quote concat }
] into append

(@string @int) -> (@char) {
	push 1
	swap
	substr
	as-char
} into getchar

[
	(@string @char) -> (@string @string) {
		over swap find
		2dup
		push 0 substr
		rot rot push 1 add swap
		dup len push 3 sub
		rot substr
	}
	(@string @string) -> (@string @string) {
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
	([..@any] @function) -> ([..@any]) {
		over len into l
		swap explode
		l push 1 add roll
		l swap
		map
		take l pack
	}
	;; map n elements of the stack ;;
	(..@any @int @function) -> (..@any) {
		push 0 swap
			(..@any @int @int @function @function) -> (..@any) {
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
    ( [..@any] @any @any ) -> ([..@any]) {
        swap
        push 1 pack
        swap

        [
            ( [] [..@any] @any @any ) -> ([..@any]) {
                drop
                drop
                swap
                drop
            }

            ( [ @any | [..@any] ] [..@any] @any @any ) -> ([..@any]) {
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

    ( .. ) -> () {
        push "Error: scan arguments mismatch!" println
    }
] into scan
;;

[
	( [@int | [..@int] ] ) -> (@bool) {
		uncon swap
		rot rot
		over over
		lt
	}
] into sort

([..@int]) -> (@int) {
	uncon swap
	( @function @int [..@int] ) -> (@int) {
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

(@string @string) -> ([..@string]) {
    into sep

    (@string) -> (..@string) {
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

(..@any @string) -> () {
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

(@any) -> () {
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

(@any) -> () {
	push 1
	swap
	sys-write
} into print

(@string @int) -> () {
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
(@int) -> (@string) {
	push 1
	sys-read
} into read-char
