include io

fun fib {
	into var limit
	push 1 into var idx
	push 1 0
	call __fib
	push limit drop
	drop
}

fun __fib{
	over
	add
	swap
	push idx
	push 1
	add
	dup
	push limit
	dup
	into var limit
	lt if {
		into var idx
		call __fib
	} else {
		drop
	}
}

;;push 90 call fib call print;;

fun mod {
    ;; Stack expects: [a, b] ;;
    over over   ;; [a, b, a, b] ;;
    div mul     ;; [a, b, (a/b)*b] ;;
    sub         ;; [a, a - (a/b)*b] -> Wait, we need to drop the original b ;;
}

fun collatz_step {
    ;; Stack expects: [n, count] ;;
    over
	push 1
	eq if {
        ;; if n == 1, we are done. Return count. ;;
        swap drop
    } else {
        ;; Check if even or odd ;;
        over push 2 call mod
		push 0
        eq if {
            ;; EVEN: n / 2 ;;
            swap push 2 div swap
        } else {
            ;; ODD: 3n + 1 ;;
            swap push 3 mul push 1 add swap
        }
        ;; Increment count and recurse ;;
        push 1 add
        call collatz_step
    }
}

;; Run Collatz for 837,799 starting with step count 0 ;;
;;
push 837799
push 0
call collatz_step
pop
;;
fun ackermann {
    ;; Stack expects: [m, n] ;;
    over
	push 0
	eq if {
        ;; if m == 0: return n + 1 ;;
        swap drop       ;; drop m ;;
        push 1 add      ;; n + 1 ;;
    } else {
		dup
		push 0
		eq if {
            ;; if n == 0: return A(m - 1, 1) ;;
            drop            ;; drop n ;;
            push 1 swap     ;; push 1, swap to get [1, m] ;;
            push 1 sub      ;; [1, m - 1] ;;
            swap            ;; [m - 1, 1] ;;
            call ackermann
        } else {
            ;; if m > 0 and n > 0: return A(m - 1, A(m, n - 1)) ;;
            ;; Current stack: [m, n] ;;
            over push 1 sub ;; [m, n, m - 1] ;;
            swap            ;; [m, m - 1, n] ;;
            rot             ;; [m - 1, n, m] ;;
            swap            ;; [m - 1, m, n] ;;
            push 1 sub      ;; [m - 1, m, n - 1] ;;
            call ackermann  ;; [m - 1, A(m, n - 1)] ;;
            call ackermann  ;; Returns final result ;;
        }
    }
}

;; Start with something small. A(3, 4) requires over 10,000 nested function calls! ;;
;; A(4, 1) will likely crash your VM. ;;
push 3 
push 4 
call ackermann
pop
