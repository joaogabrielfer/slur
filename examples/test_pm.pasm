include io

(0..<15) {
	drop
	push "menor que 15" println
} into f


[
(@int @int) { add }
(@int) {
		into c
		(@int) {
			take c
			add
		}
	}
] into c_add

;;
push 1 c_add
push 2 swap eval
print
;;

([@char | @string]) {
	swap
	println
} into print_char

push "teste"
print_char
print_char
print_char
