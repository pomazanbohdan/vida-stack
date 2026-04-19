	.file	"cbb.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/cbb.c"
	.section	.text.CRYPTO_bswap2,"ax",@progbits
	.type	CRYPTO_bswap2, @function
CRYPTO_bswap2:
.LFB88:
	.file 2 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/../internal.h"
	.loc 2 846 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movl	%edi, %eax
	movw	%ax, -4(%rbp)
	.loc 2 847 10
	movzwl	-4(%rbp), %eax
	xchgb	%ah, %al
	.loc 2 848 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE88:
	.size	CRYPTO_bswap2, .-CRYPTO_bswap2
	.section	.text.CRYPTO_bswap4,"ax",@progbits
	.type	CRYPTO_bswap4, @function
CRYPTO_bswap4:
.LFB89:
	.loc 2 850 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movl	%edi, -4(%rbp)
	.loc 2 851 10
	movbel	-4(%rbp), %eax
	.loc 2 852 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE89:
	.size	CRYPTO_bswap4, .-CRYPTO_bswap4
	.section	.text.CRYPTO_bswap8,"ax",@progbits
	.type	CRYPTO_bswap8, @function
CRYPTO_bswap8:
.LFB90:
	.loc 2 854 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	.loc 2 855 10
	movbeq	-8(%rbp), %rax
	.loc 2 856 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE90:
	.size	CRYPTO_bswap8, .-CRYPTO_bswap8
	.section	.text.OPENSSL_memcmp,"ax",@progbits
	.type	OPENSSL_memcmp, @function
OPENSSL_memcmp:
.LFB93:
	.loc 2 948 76
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 2 949 6
	cmpq	$0, -24(%rbp)
	jne	.L8
	.loc 2 950 12
	movl	$0, %eax
	jmp	.L9
.L8:
	.loc 2 953 10
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	memcmp@PLT
.L9:
	.loc 2 954 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE93:
	.size	OPENSSL_memcmp, .-OPENSSL_memcmp
	.section	.text.OPENSSL_memcpy,"ax",@progbits
	.type	OPENSSL_memcpy, @function
OPENSSL_memcpy:
.LFB94:
	.loc 2 956 74
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 2 957 6
	cmpq	$0, -24(%rbp)
	jne	.L11
	.loc 2 958 12
	movq	-8(%rbp), %rax
	jmp	.L12
.L11:
	.loc 2 961 10
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	memcpy@PLT
.L12:
	.loc 2 962 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE94:
	.size	OPENSSL_memcpy, .-OPENSSL_memcpy
	.section	.text.OPENSSL_memmove,"ax",@progbits
	.type	OPENSSL_memmove, @function
OPENSSL_memmove:
.LFB95:
	.loc 2 964 75
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 2 965 6
	cmpq	$0, -24(%rbp)
	jne	.L14
	.loc 2 966 12
	movq	-8(%rbp), %rax
	jmp	.L15
.L14:
	.loc 2 969 10
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	memmove@PLT
.L15:
	.loc 2 970 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE95:
	.size	OPENSSL_memmove, .-OPENSSL_memmove
	.section	.text.OPENSSL_memset,"ax",@progbits
	.type	OPENSSL_memset, @function
OPENSSL_memset:
.LFB96:
	.loc 2 972 64
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, -12(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 2 973 6
	cmpq	$0, -24(%rbp)
	jne	.L17
	.loc 2 974 12
	movq	-8(%rbp), %rax
	jmp	.L18
.L17:
	.loc 2 977 10
	movq	-24(%rbp), %rdx
	movl	-12(%rbp), %ecx
	movq	-8(%rbp), %rax
	movl	%ecx, %esi
	movq	%rax, %rdi
	call	memset@PLT
.L18:
	.loc 2 978 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE96:
	.size	OPENSSL_memset, .-OPENSSL_memset
	.section	.text.aws_lc_0_38_0_CBB_zero,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_zero
	.type	aws_lc_0_38_0_CBB_zero, @function
aws_lc_0_38_0_CBB_zero:
.LFB128:
	.loc 1 27 25
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	.loc 1 28 3
	movq	-8(%rbp), %rax
	movl	$48, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 29 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE128:
	.size	aws_lc_0_38_0_CBB_zero, .-aws_lc_0_38_0_CBB_zero
	.section	.text.cbb_init,"ax",@progbits
	.type	cbb_init, @function
cbb_init:
.LFB129:
	.loc 1 31 74
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	movl	%ecx, -28(%rbp)
	.loc 1 32 17
	movq	-8(%rbp), %rax
	movb	$0, 8(%rax)
	.loc 1 33 14
	movq	-8(%rbp), %rax
	movq	$0, (%rax)
	.loc 1 34 19
	movq	-8(%rbp), %rax
	movq	-16(%rbp), %rdx
	movq	%rdx, 16(%rax)
	.loc 1 35 19
	movq	-8(%rbp), %rax
	movq	$0, 24(%rax)
	.loc 1 36 19
	movq	-8(%rbp), %rax
	movq	-24(%rbp), %rdx
	movq	%rdx, 32(%rax)
	.loc 1 37 26
	movl	-28(%rbp), %eax
	andl	$1, %eax
	movq	-8(%rbp), %rdx
	andl	$1, %eax
	movl	%eax, %ecx
	movzbl	40(%rdx), %eax
	andl	$-2, %eax
	orl	%ecx, %eax
	movb	%al, 40(%rdx)
	.loc 1 38 21
	movq	-8(%rbp), %rax
	movzbl	40(%rax), %edx
	andl	$-3, %edx
	movb	%dl, 40(%rax)
	.loc 1 39 1
	nop
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE129:
	.size	cbb_init, .-cbb_init
	.section	.text.aws_lc_0_38_0_CBB_init,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_init
	.type	aws_lc_0_38_0_CBB_init, @function
aws_lc_0_38_0_CBB_init:
.LFB130:
	.loc 1 41 49
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 42 3
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_zero@PLT
	.loc 1 44 18
	movq	-32(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_malloc@PLT
	movq	%rax, -8(%rbp)
	.loc 1 45 6
	cmpq	$0, -32(%rbp)
	je	.L22
	.loc 1 45 28 discriminator 1
	cmpq	$0, -8(%rbp)
	jne	.L22
	.loc 1 46 12
	movl	$0, %eax
	jmp	.L23
.L22:
	.loc 1 49 3
	movq	-32(%rbp), %rdx
	movq	-8(%rbp), %rsi
	movq	-24(%rbp), %rax
	movl	$1, %ecx
	movq	%rax, %rdi
	call	cbb_init
	.loc 1 50 10
	movl	$1, %eax
.L23:
	.loc 1 51 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE130:
	.size	aws_lc_0_38_0_CBB_init, .-aws_lc_0_38_0_CBB_init
	.section	.text.aws_lc_0_38_0_CBB_init_fixed,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_init_fixed
	.type	aws_lc_0_38_0_CBB_init_fixed, @function
aws_lc_0_38_0_CBB_init_fixed:
.LFB131:
	.loc 1 53 56
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 1 54 3
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_zero@PLT
	.loc 1 55 3
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rsi
	movq	-8(%rbp), %rax
	movl	$0, %ecx
	movq	%rax, %rdi
	call	cbb_init
	.loc 1 56 10
	movl	$1, %eax
	.loc 1 57 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE131:
	.size	aws_lc_0_38_0_CBB_init_fixed, .-aws_lc_0_38_0_CBB_init_fixed
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/cbb.c"
.LC1:
	.string	"!cbb->is_child"
	.section	.text.aws_lc_0_38_0_CBB_cleanup,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_cleanup
	.type	aws_lc_0_38_0_CBB_cleanup, @function
aws_lc_0_38_0_CBB_cleanup:
.LFB132:
	.loc 1 59 28
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	.loc 1 62 3
	movq	-8(%rbp), %rax
	movzbl	8(%rax), %eax
	testb	%al, %al
	je	.L27
	.loc 1 62 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.6(%rip), %rax
	movq	%rax, %rcx
	movl	$62, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC1(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L27:
	.loc 1 63 10 is_stmt 1
	movq	-8(%rbp), %rax
	movzbl	8(%rax), %eax
	.loc 1 63 6
	testb	%al, %al
	jne	.L30
	.loc 1 67 7
	movq	-8(%rbp), %rax
	movzbl	40(%rax), %eax
	andl	$1, %eax
	.loc 1 67 6
	testb	%al, %al
	je	.L26
	.loc 1 68 29
	movq	-8(%rbp), %rax
	movq	16(%rax), %rax
	.loc 1 68 5
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_free@PLT
	jmp	.L26
.L30:
	.loc 1 64 5
	nop
.L26:
	.loc 1 70 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE132:
	.size	aws_lc_0_38_0_CBB_cleanup, .-aws_lc_0_38_0_CBB_cleanup
	.section	.text.cbb_buffer_reserve,"ax",@progbits
	.type	cbb_buffer_reserve, @function
cbb_buffer_reserve:
.LFB133:
	.loc 1 73 43
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	movq	%rdx, -56(%rbp)
	.loc 1 74 6
	cmpq	$0, -40(%rbp)
	jne	.L32
	.loc 1 75 12
	movl	$0, %eax
	jmp	.L33
.L32:
	.loc 1 78 23
	movq	-40(%rbp), %rax
	movq	8(%rax), %rdx
	.loc 1 78 10
	movq	-56(%rbp), %rax
	addq	%rdx, %rax
	movq	%rax, -16(%rbp)
	.loc 1 79 20
	movq	-40(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 79 6
	cmpq	%rax, -16(%rbp)
	jnb	.L34
	.loc 1 81 5
	movl	$81, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$69, %edx
	movl	$0, %esi
	movl	$14, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 82 5
	jmp	.L35
.L34:
	.loc 1 85 20
	movq	-40(%rbp), %rax
	movq	16(%rax), %rax
	.loc 1 85 6
	cmpq	-16(%rbp), %rax
	jnb	.L36
.LBB2:
	.loc 1 86 9
	movq	-40(%rbp), %rax
	movzbl	24(%rax), %eax
	andl	$1, %eax
	.loc 1 86 8
	testb	%al, %al
	jne	.L37
	.loc 1 87 7
	movl	$87, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$69, %edx
	movl	$0, %esi
	movl	$14, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 88 7
	jmp	.L35
.L37:
	.loc 1 91 25
	movq	-40(%rbp), %rax
	movq	16(%rax), %rax
	.loc 1 91 12
	addq	%rax, %rax
	movq	%rax, -8(%rbp)
	.loc 1 92 22
	movq	-40(%rbp), %rax
	movq	16(%rax), %rax
	.loc 1 92 8
	cmpq	%rax, -8(%rbp)
	jb	.L38
	.loc 1 92 28 discriminator 1
	movq	-8(%rbp), %rax
	cmpq	-16(%rbp), %rax
	jnb	.L39
.L38:
	.loc 1 93 14
	movq	-16(%rbp), %rax
	movq	%rax, -8(%rbp)
.L39:
	.loc 1 95 43
	movq	-40(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 95 23
	movq	-8(%rbp), %rdx
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_realloc@PLT
	movq	%rax, -24(%rbp)
	.loc 1 96 8
	cmpq	$0, -24(%rbp)
	je	.L42
	.loc 1 100 15
	movq	-40(%rbp), %rax
	movq	-24(%rbp), %rdx
	movq	%rdx, (%rax)
	.loc 1 101 15
	movq	-40(%rbp), %rax
	movq	-8(%rbp), %rdx
	movq	%rdx, 16(%rax)
.L36:
.LBE2:
	.loc 1 104 6
	cmpq	$0, -48(%rbp)
	je	.L41
	.loc 1 105 16
	movq	-40(%rbp), %rax
	movq	(%rax), %rdx
	.loc 1 105 28
	movq	-40(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 105 22
	addq	%rax, %rdx
	.loc 1 105 10
	movq	-48(%rbp), %rax
	movq	%rdx, (%rax)
.L41:
	.loc 1 108 10
	movl	$1, %eax
	jmp	.L33
.L42:
.LBB3:
	.loc 1 97 7
	nop
.L35:
.LBE3:
	.loc 1 111 15
	movq	-40(%rbp), %rax
	movzbl	24(%rax), %edx
	orl	$2, %edx
	movb	%dl, 24(%rax)
	.loc 1 112 10
	movl	$0, %eax
.L33:
	.loc 1 113 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE133:
	.size	cbb_buffer_reserve, .-cbb_buffer_reserve
	.section	.text.cbb_buffer_add,"ax",@progbits
	.type	cbb_buffer_add, @function
cbb_buffer_add:
.LFB134:
	.loc 1 116 39
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 1 117 8
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbb_buffer_reserve
	.loc 1 117 6 discriminator 1
	testl	%eax, %eax
	jne	.L44
	.loc 1 118 12
	movl	$0, %eax
	jmp	.L45
.L44:
	.loc 1 121 7
	movq	-8(%rbp), %rax
	movq	8(%rax), %rdx
	.loc 1 121 13
	movq	-24(%rbp), %rax
	addq	%rax, %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, 8(%rax)
	.loc 1 122 10
	movl	$1, %eax
.L45:
	.loc 1 123 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE134:
	.size	cbb_buffer_add, .-cbb_buffer_add
	.section	.text.aws_lc_0_38_0_CBB_finish,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_finish
	.type	aws_lc_0_38_0_CBB_finish, @function
aws_lc_0_38_0_CBB_finish:
.LFB135:
	.loc 1 125 63
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 1 126 10
	movq	-8(%rbp), %rax
	movzbl	8(%rax), %eax
	.loc 1 126 6
	testb	%al, %al
	je	.L47
	.loc 1 127 5
	movl	$127, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$66, %edx
	movl	$0, %esi
	movl	$14, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 128 12
	movl	$0, %eax
	jmp	.L48
.L47:
	.loc 1 131 8
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 131 6 discriminator 1
	testl	%eax, %eax
	jne	.L49
	.loc 1 132 12
	movl	$0, %eax
	jmp	.L48
.L49:
	.loc 1 135 7
	movq	-8(%rbp), %rax
	movzbl	40(%rax), %eax
	andl	$1, %eax
	.loc 1 135 6
	testb	%al, %al
	je	.L50
	.loc 1 135 30 discriminator 1
	cmpq	$0, -16(%rbp)
	je	.L51
	.loc 1 135 51 discriminator 2
	cmpq	$0, -24(%rbp)
	jne	.L50
.L51:
	.loc 1 137 12
	movl	$0, %eax
	jmp	.L48
.L50:
	.loc 1 140 6
	cmpq	$0, -16(%rbp)
	je	.L52
	.loc 1 141 28
	movq	-8(%rbp), %rax
	movq	16(%rax), %rdx
	.loc 1 141 15
	movq	-16(%rbp), %rax
	movq	%rdx, (%rax)
.L52:
	.loc 1 143 6
	cmpq	$0, -24(%rbp)
	je	.L53
	.loc 1 144 27
	movq	-8(%rbp), %rax
	movq	24(%rax), %rdx
	.loc 1 144 14
	movq	-24(%rbp), %rax
	movq	%rdx, (%rax)
.L53:
	.loc 1 146 19
	movq	-8(%rbp), %rax
	movq	$0, 16(%rax)
	.loc 1 147 3
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_cleanup@PLT
	.loc 1 148 10
	movl	$1, %eax
.L48:
	.loc 1 149 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE135:
	.size	aws_lc_0_38_0_CBB_finish, .-aws_lc_0_38_0_CBB_finish
	.section	.text.cbb_get_base,"ax",@progbits
	.type	cbb_get_base, @function
cbb_get_base:
.LFB136:
	.loc 1 151 53
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	.loc 1 152 10
	movq	-8(%rbp), %rax
	movzbl	8(%rax), %eax
	.loc 1 152 6
	testb	%al, %al
	je	.L55
	.loc 1 153 24
	movq	-8(%rbp), %rax
	movq	16(%rax), %rax
	jmp	.L56
.L55:
	.loc 1 155 10
	movq	-8(%rbp), %rax
	addq	$16, %rax
.L56:
	.loc 1 156 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE136:
	.size	cbb_get_base, .-cbb_get_base
	.section	.text.cbb_on_error,"ax",@progbits
	.type	cbb_on_error, @function
cbb_on_error:
.LFB137:
	.loc 1 158 36
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$8, %rsp
	movq	%rdi, -8(%rbp)
	.loc 1 173 3
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_get_base
	.loc 1 173 28 discriminator 1
	movzbl	24(%rax), %edx
	orl	$2, %edx
	movb	%dl, 24(%rax)
	.loc 1 178 14
	movq	-8(%rbp), %rax
	movq	$0, (%rax)
	.loc 1 179 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE137:
	.size	cbb_on_error, .-cbb_on_error
	.section	.rodata
.LC2:
	.string	"cbb->child->is_child"
.LC3:
	.string	"child->base == base"
.LC4:
	.string	"child->pending_len_len == 1"
	.section	.text.aws_lc_0_38_0_CBB_flush,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_flush
	.type	aws_lc_0_38_0_CBB_flush, @function
aws_lc_0_38_0_CBB_flush:
.LFB138:
	.loc 1 184 25
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$80, %rsp
	movq	%rdi, -72(%rbp)
	.loc 1 188 32
	movq	-72(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_get_base
	movq	%rax, -32(%rbp)
	.loc 1 189 6
	cmpq	$0, -32(%rbp)
	je	.L59
	.loc 1 189 20 discriminator 1
	movq	-32(%rbp), %rax
	movzbl	24(%rax), %eax
	andl	$2, %eax
	testb	%al, %al
	je	.L60
.L59:
	.loc 1 190 12
	movl	$0, %eax
	jmp	.L61
.L60:
	.loc 1 193 10
	movq	-72(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 193 6
	testq	%rax, %rax
	jne	.L62
	.loc 1 195 12
	movl	$1, %eax
	jmp	.L61
.L62:
	.loc 1 198 3
	movq	-72(%rbp), %rax
	movq	(%rax), %rax
	movzbl	8(%rax), %eax
	testb	%al, %al
	jne	.L63
	.loc 1 198 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.5(%rip), %rax
	movq	%rax, %rcx
	movl	$198, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC2(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L63:
	.loc 1 199 36 is_stmt 1
	movq	-72(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 199 24
	addq	$16, %rax
	movq	%rax, -40(%rbp)
	.loc 1 200 3
	movq	-40(%rbp), %rax
	movq	(%rax), %rax
	cmpq	%rax, -32(%rbp)
	je	.L64
	.loc 1 200 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.5(%rip), %rax
	movq	%rax, %rcx
	movl	$200, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC3(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L64:
	.loc 1 201 29 is_stmt 1
	movq	-40(%rbp), %rax
	movq	8(%rax), %rdx
	.loc 1 201 45
	movq	-40(%rbp), %rax
	movzbl	16(%rax), %eax
	movzbl	%al, %eax
	.loc 1 201 10
	addq	%rdx, %rax
	movq	%rax, -48(%rbp)
	.loc 1 203 8
	movq	-72(%rbp), %rax
	movq	(%rax), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush.localalias
	.loc 1 203 6 discriminator 1
	testl	%eax, %eax
	je	.L81
	.loc 1 204 26
	movq	-40(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 203 30 discriminator 1
	cmpq	%rax, -48(%rbp)
	jb	.L81
	.loc 1 205 11
	movq	-32(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 204 35
	cmpq	-48(%rbp), %rax
	jb	.L81
	.loc 1 209 20
	movq	-32(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 209 10
	subq	-48(%rbp), %rax
	movq	%rax, -8(%rbp)
	.loc 1 211 7
	movq	-40(%rbp), %rax
	movzbl	17(%rax), %eax
	andl	$1, %eax
	.loc 1 211 6
	testb	%al, %al
	je	.L68
.LBB4:
	.loc 1 218 5
	movq	-40(%rbp), %rax
	movzbl	16(%rax), %eax
	cmpb	$1, %al
	je	.L69
	.loc 1 218 5 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.5(%rip), %rax
	movq	%rax, %rcx
	movl	$218, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC4(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L69:
	.loc 1 220 8 is_stmt 1
	movl	$4294967294, %eax
	cmpq	-8(%rbp), %rax
	jnb	.L70
	.loc 1 221 7
	movl	$221, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$69, %edx
	movl	$0, %esi
	movl	$14, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 223 7
	jmp	.L67
.L70:
	.loc 1 224 15
	cmpq	$16777215, -8(%rbp)
	jbe	.L71
	.loc 1 225 15
	movb	$5, -9(%rbp)
	.loc 1 226 27
	movb	$-124, -10(%rbp)
	jmp	.L72
.L71:
	.loc 1 227 15
	cmpq	$65535, -8(%rbp)
	jbe	.L73
	.loc 1 228 15
	movb	$4, -9(%rbp)
	.loc 1 229 27
	movb	$-125, -10(%rbp)
	jmp	.L72
.L73:
	.loc 1 230 15
	cmpq	$255, -8(%rbp)
	jbe	.L74
	.loc 1 231 15
	movb	$3, -9(%rbp)
	.loc 1 232 27
	movb	$-126, -10(%rbp)
	jmp	.L72
.L74:
	.loc 1 233 15
	cmpq	$127, -8(%rbp)
	jbe	.L75
	.loc 1 234 15
	movb	$2, -9(%rbp)
	.loc 1 235 27
	movb	$-127, -10(%rbp)
	jmp	.L72
.L75:
	.loc 1 237 15
	movb	$1, -9(%rbp)
	.loc 1 238 27
	movq	-8(%rbp), %rax
	movb	%al, -10(%rbp)
	.loc 1 239 11
	movq	$0, -8(%rbp)
.L72:
	.loc 1 242 8
	cmpb	$1, -9(%rbp)
	je	.L76
.LBB5:
	.loc 1 244 36
	movzbl	-9(%rbp), %eax
	subl	$1, %eax
	.loc 1 244 14
	cltq
	movq	%rax, -56(%rbp)
	.loc 1 245 12
	movq	-56(%rbp), %rdx
	movq	-32(%rbp), %rax
	movl	$0, %esi
	movq	%rax, %rdi
	call	cbb_buffer_add
	.loc 1 245 10 discriminator 1
	testl	%eax, %eax
	je	.L82
	.loc 1 249 27
	movq	-32(%rbp), %rax
	movq	(%rax), %rdx
	.loc 1 249 33
	movq	-48(%rbp), %rax
	leaq	(%rdx,%rax), %rsi
	.loc 1 248 27
	movq	-32(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 248 47
	movq	-48(%rbp), %rcx
	movq	-56(%rbp), %rdx
	addq	%rcx, %rdx
	leaq	(%rax,%rdx), %rcx
	.loc 1 248 7
	movq	-8(%rbp), %rax
	movq	%rax, %rdx
	movq	%rcx, %rdi
	call	OPENSSL_memmove
.L76:
.LBE5:
	.loc 1 251 9
	movq	-32(%rbp), %rax
	movq	(%rax), %rsi
	.loc 1 251 20
	movq	-40(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 251 28
	leaq	1(%rax), %rcx
	movq	-40(%rbp), %rdx
	movq	%rcx, 8(%rdx)
	.loc 1 251 14
	leaq	(%rsi,%rax), %rdx
	.loc 1 251 32
	movzbl	-10(%rbp), %eax
	movb	%al, (%rdx)
	.loc 1 252 38
	movzbl	-9(%rbp), %eax
	leal	-1(%rax), %edx
	.loc 1 252 28
	movq	-40(%rbp), %rax
	movb	%dl, 16(%rax)
.L68:
.LBE4:
.LBB7:
	.loc 1 255 24
	movq	-40(%rbp), %rax
	movzbl	16(%rax), %eax
	movzbl	%al, %eax
	.loc 1 255 42
	subl	$1, %eax
	.loc 1 255 15
	cltq
	movq	%rax, -24(%rbp)
	.loc 1 255 3
	jmp	.L78
.L79:
	.loc 1 256 9
	movq	-32(%rbp), %rax
	movq	(%rax), %rdx
	.loc 1 256 20
	movq	-40(%rbp), %rax
	movq	8(%rax), %rcx
	.loc 1 256 29
	movq	-24(%rbp), %rax
	addq	%rcx, %rax
	.loc 1 256 14
	addq	%rdx, %rax
	.loc 1 256 36
	movq	-8(%rbp), %rdx
	.loc 1 256 34
	movb	%dl, (%rax)
	.loc 1 257 9
	shrq	$8, -8(%rbp)
	.loc 1 255 76 discriminator 3
	subq	$1, -24(%rbp)
.L78:
	.loc 1 255 56 discriminator 1
	movq	-40(%rbp), %rax
	movzbl	16(%rax), %eax
	movzbl	%al, %eax
	.loc 1 255 49 discriminator 1
	cmpq	%rax, -24(%rbp)
	jb	.L79
.LBE7:
	.loc 1 259 6
	cmpq	$0, -8(%rbp)
	je	.L80
	.loc 1 260 5
	movl	$260, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$69, %edx
	movl	$0, %esi
	movl	$14, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 261 5
	jmp	.L67
.L80:
	.loc 1 264 15
	movq	-40(%rbp), %rax
	movq	$0, (%rax)
	.loc 1 265 14
	movq	-72(%rbp), %rax
	movq	$0, (%rax)
	.loc 1 267 10
	movl	$1, %eax
	jmp	.L61
.L81:
	.loc 1 206 5
	nop
	jmp	.L67
.L82:
.LBB8:
.LBB6:
	.loc 1 246 9
	nop
.L67:
.LBE6:
.LBE8:
	.loc 1 270 3
	movq	-72(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_on_error
	.loc 1 271 10
	movl	$0, %eax
.L61:
	.loc 1 272 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE138:
	.size	aws_lc_0_38_0_CBB_flush, .-aws_lc_0_38_0_CBB_flush
	.set	aws_lc_0_38_0_CBB_flush.localalias,aws_lc_0_38_0_CBB_flush
	.section	.rodata
.LC5:
	.string	"cbb->child == NULL"
	.section	.text.aws_lc_0_38_0_CBB_data,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_data
	.type	aws_lc_0_38_0_CBB_data, @function
aws_lc_0_38_0_CBB_data:
.LFB139:
	.loc 1 274 41
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	.loc 1 275 3
	movq	-8(%rbp), %rax
	movq	(%rax), %rax
	testq	%rax, %rax
	je	.L84
	.loc 1 275 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.4(%rip), %rax
	movq	%rax, %rcx
	movl	$275, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC5(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L84:
	.loc 1 276 10 is_stmt 1
	movq	-8(%rbp), %rax
	movzbl	8(%rax), %eax
	.loc 1 276 6
	testb	%al, %al
	je	.L85
	.loc 1 277 24
	movq	-8(%rbp), %rax
	movq	16(%rax), %rax
	.loc 1 277 29
	movq	(%rax), %rdx
	.loc 1 277 49
	movq	-8(%rbp), %rax
	movq	24(%rax), %rcx
	.loc 1 278 24
	movq	-8(%rbp), %rax
	movzbl	32(%rax), %eax
	movzbl	%al, %eax
	.loc 1 277 57
	addq	%rcx, %rax
	addq	%rdx, %rax
	jmp	.L86
.L85:
	.loc 1 280 21
	movq	-8(%rbp), %rax
	movq	16(%rax), %rax
.L86:
	.loc 1 281 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE139:
	.size	aws_lc_0_38_0_CBB_data, .-aws_lc_0_38_0_CBB_data
	.section	.rodata
	.align 8
.LC6:
	.string	"cbb->u.child.offset + cbb->u.child.pending_len_len <= cbb->u.child.base->len"
	.section	.text.aws_lc_0_38_0_CBB_len,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_len
	.type	aws_lc_0_38_0_CBB_len, @function
aws_lc_0_38_0_CBB_len:
.LFB140:
	.loc 1 283 32
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	.loc 1 284 3
	movq	-8(%rbp), %rax
	movq	(%rax), %rax
	testq	%rax, %rax
	je	.L88
	.loc 1 284 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.3(%rip), %rax
	movq	%rax, %rcx
	movl	$284, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC5(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L88:
	.loc 1 285 10 is_stmt 1
	movq	-8(%rbp), %rax
	movzbl	8(%rax), %eax
	.loc 1 285 6
	testb	%al, %al
	je	.L89
	.loc 1 286 5
	movq	-8(%rbp), %rax
	movq	24(%rax), %rdx
	movq	-8(%rbp), %rax
	movzbl	32(%rax), %eax
	movzbl	%al, %eax
	addq	%rax, %rdx
	movq	-8(%rbp), %rax
	movq	16(%rax), %rax
	movq	8(%rax), %rax
	cmpq	%rdx, %rax
	jnb	.L90
	.loc 1 286 5 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.3(%rip), %rax
	movq	%rax, %rcx
	movl	$286, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC6(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L90:
	.loc 1 288 24 is_stmt 1
	movq	-8(%rbp), %rax
	movq	16(%rax), %rax
	.loc 1 288 29
	movq	8(%rax), %rdx
	.loc 1 288 49
	movq	-8(%rbp), %rax
	movq	24(%rax), %rax
	.loc 1 288 35
	movq	%rdx, %rcx
	subq	%rax, %rcx
	.loc 1 289 24
	movq	-8(%rbp), %rax
	movzbl	32(%rax), %eax
	movzbl	%al, %edx
	.loc 1 288 57
	movq	%rcx, %rax
	subq	%rdx, %rax
	jmp	.L91
.L89:
	.loc 1 291 21
	movq	-8(%rbp), %rax
	movq	24(%rax), %rax
.L91:
	.loc 1 292 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE140:
	.size	aws_lc_0_38_0_CBB_len, .-aws_lc_0_38_0_CBB_len
	.section	.rodata
.LC7:
	.string	"!is_asn1 || len_len == 1"
	.section	.text.cbb_add_child,"ax",@progbits
	.type	cbb_add_child, @function
cbb_add_child:
.LFB141:
	.loc 1 295 39
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	movl	%edx, %eax
	movl	%ecx, -56(%rbp)
	movb	%al, -52(%rbp)
	.loc 1 296 3
	movq	-40(%rbp), %rax
	movq	(%rax), %rax
	testq	%rax, %rax
	je	.L93
	.loc 1 296 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.2(%rip), %rax
	movq	%rax, %rcx
	movl	$296, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC5(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L93:
	.loc 1 297 3 is_stmt 1
	cmpl	$0, -56(%rbp)
	je	.L94
	.loc 1 297 3 is_stmt 0 discriminator 1
	cmpb	$1, -52(%rbp)
	je	.L94
	.loc 1 297 3 discriminator 2
	leaq	__PRETTY_FUNCTION__.2(%rip), %rax
	movq	%rax, %rcx
	movl	$297, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC7(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L94:
	.loc 1 298 32 is_stmt 1
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_get_base
	movq	%rax, -8(%rbp)
	.loc 1 299 10
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	movq	%rax, -16(%rbp)
	.loc 1 303 8
	movzbl	-52(%rbp), %edx
	leaq	-24(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbb_buffer_add
	.loc 1 303 6 discriminator 1
	testl	%eax, %eax
	jne	.L95
	.loc 1 304 12
	movl	$0, %eax
	jmp	.L97
.L95:
	.loc 1 306 3
	movzbl	-52(%rbp), %edx
	movq	-24(%rbp), %rax
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 308 3
	movq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_zero@PLT
	.loc 1 309 23
	movq	-48(%rbp), %rax
	movb	$1, 8(%rax)
	.loc 1 310 27
	movq	-48(%rbp), %rax
	movq	-8(%rbp), %rdx
	movq	%rdx, 16(%rax)
	.loc 1 311 29
	movq	-48(%rbp), %rax
	movq	-16(%rbp), %rdx
	movq	%rdx, 24(%rax)
	.loc 1 312 38
	movq	-48(%rbp), %rax
	movzbl	-52(%rbp), %edx
	movb	%dl, 32(%rax)
	.loc 1 313 38
	movl	-56(%rbp), %eax
	andl	$1, %eax
	movq	-48(%rbp), %rdx
	andl	$1, %eax
	movl	%eax, %ecx
	movzbl	33(%rdx), %eax
	andl	$-2, %eax
	orl	%ecx, %eax
	movb	%al, 33(%rdx)
	.loc 1 314 14
	movq	-40(%rbp), %rax
	movq	-48(%rbp), %rdx
	movq	%rdx, (%rax)
	.loc 1 315 10
	movl	$1, %eax
.L97:
	.loc 1 316 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE141:
	.size	cbb_add_child, .-cbb_add_child
	.section	.text.cbb_add_length_prefixed,"ax",@progbits
	.type	cbb_add_length_prefixed, @function
cbb_add_length_prefixed:
.LFB142:
	.loc 1 319 53
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movl	%edx, %eax
	movb	%al, -20(%rbp)
	.loc 1 320 8
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 320 6 discriminator 1
	testl	%eax, %eax
	jne	.L99
	.loc 1 321 12
	movl	$0, %eax
	jmp	.L100
.L99:
	.loc 1 324 10
	movzbl	-20(%rbp), %edx
	movq	-16(%rbp), %rsi
	movq	-8(%rbp), %rax
	movl	$0, %ecx
	movq	%rax, %rdi
	call	cbb_add_child
.L100:
	.loc 1 325 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE142:
	.size	cbb_add_length_prefixed, .-cbb_add_length_prefixed
	.section	.text.aws_lc_0_38_0_CBB_add_u8_length_prefixed,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u8_length_prefixed
	.type	aws_lc_0_38_0_CBB_add_u8_length_prefixed, @function
aws_lc_0_38_0_CBB_add_u8_length_prefixed:
.LFB143:
	.loc 1 327 61
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 328 10
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$1, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbb_add_length_prefixed
	.loc 1 329 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE143:
	.size	aws_lc_0_38_0_CBB_add_u8_length_prefixed, .-aws_lc_0_38_0_CBB_add_u8_length_prefixed
	.section	.text.aws_lc_0_38_0_CBB_add_u16_length_prefixed,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u16_length_prefixed
	.type	aws_lc_0_38_0_CBB_add_u16_length_prefixed, @function
aws_lc_0_38_0_CBB_add_u16_length_prefixed:
.LFB144:
	.loc 1 331 62
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 332 10
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$2, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbb_add_length_prefixed
	.loc 1 333 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE144:
	.size	aws_lc_0_38_0_CBB_add_u16_length_prefixed, .-aws_lc_0_38_0_CBB_add_u16_length_prefixed
	.section	.text.aws_lc_0_38_0_CBB_add_u24_length_prefixed,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u24_length_prefixed
	.type	aws_lc_0_38_0_CBB_add_u24_length_prefixed, @function
aws_lc_0_38_0_CBB_add_u24_length_prefixed:
.LFB145:
	.loc 1 335 62
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 336 10
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$3, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbb_add_length_prefixed
	.loc 1 337 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE145:
	.size	aws_lc_0_38_0_CBB_add_u24_length_prefixed, .-aws_lc_0_38_0_CBB_add_u24_length_prefixed
	.section	.text.add_base128_integer,"ax",@progbits
	.type	add_base128_integer, @function
add_base128_integer:
.LFB146:
	.loc 1 342 54
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	.loc 1 343 12
	movl	$0, -4(%rbp)
	.loc 1 344 12
	movq	-48(%rbp), %rax
	movq	%rax, -16(%rbp)
	.loc 1 345 9
	jmp	.L108
.L109:
	.loc 1 346 12
	addl	$1, -4(%rbp)
	.loc 1 347 10
	shrq	$7, -16(%rbp)
.L108:
	.loc 1 345 15
	cmpq	$0, -16(%rbp)
	jne	.L109
	.loc 1 349 6
	cmpl	$0, -4(%rbp)
	jne	.L110
	.loc 1 350 13
	movl	$1, -4(%rbp)
.L110:
.LBB9:
	.loc 1 352 17
	movl	-4(%rbp), %eax
	subl	$1, %eax
	movl	%eax, -20(%rbp)
	.loc 1 352 3
	jmp	.L111
.L115:
.LBB10:
	.loc 1 353 29
	movl	-20(%rbp), %edx
	movl	%edx, %eax
	sall	$3, %eax
	subl	%edx, %eax
	.loc 1 353 23
	movq	-48(%rbp), %rdx
	shrx	%rax, %rdx, %rax
	.loc 1 353 13
	andl	$127, %eax
	movb	%al, -21(%rbp)
	.loc 1 354 8
	cmpl	$0, -20(%rbp)
	je	.L112
	.loc 1 356 12
	orb	$-128, -21(%rbp)
.L112:
	.loc 1 358 10
	movzbl	-21(%rbp), %edx
	movq	-40(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 358 8 discriminator 1
	testl	%eax, %eax
	jne	.L113
	.loc 1 359 14
	movl	$0, %eax
	jmp	.L114
.L113:
.LBE10:
	.loc 1 352 48 discriminator 2
	subl	$1, -20(%rbp)
.L111:
	.loc 1 352 36 discriminator 1
	movl	-20(%rbp), %eax
	cmpl	-4(%rbp), %eax
	jb	.L115
.LBE9:
	.loc 1 362 10
	movl	$1, %eax
.L114:
	.loc 1 363 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE146:
	.size	add_base128_integer, .-add_base128_integer
	.section	.text.aws_lc_0_38_0_CBB_add_asn1,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_asn1
	.type	aws_lc_0_38_0_CBB_add_asn1, @function
aws_lc_0_38_0_CBB_add_asn1:
.LFB147:
	.loc 1 365 65
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	movl	%edx, -36(%rbp)
	.loc 1 366 8
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 366 6 discriminator 1
	testl	%eax, %eax
	jne	.L117
	.loc 1 367 12
	movl	$0, %eax
	jmp	.L118
.L117:
	.loc 1 371 27
	movl	-36(%rbp), %eax
	shrl	$24, %eax
	.loc 1 371 11
	andl	$-32, %eax
	movb	%al, -1(%rbp)
	.loc 1 372 16
	movl	-36(%rbp), %eax
	andl	$536870911, %eax
	movl	%eax, -8(%rbp)
	.loc 1 373 6
	cmpl	$30, -8(%rbp)
	jbe	.L119
	.loc 1 375 10
	movzbl	-1(%rbp), %eax
	orl	$31, %eax
	movzbl	%al, %edx
	movq	-24(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 375 8 discriminator 1
	testl	%eax, %eax
	je	.L120
	.loc 1 376 10
	movl	-8(%rbp), %edx
	movq	-24(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	add_base128_integer
	.loc 1 375 43 discriminator 1
	testl	%eax, %eax
	jne	.L121
.L120:
	.loc 1 377 14
	movl	$0, %eax
	jmp	.L118
.L119:
	.loc 1 379 40
	movl	-8(%rbp), %eax
	orb	-1(%rbp), %al
	.loc 1 379 15
	movzbl	%al, %edx
	movq	-24(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 379 13 discriminator 1
	testl	%eax, %eax
	jne	.L121
	.loc 1 380 12
	movl	$0, %eax
	jmp	.L118
.L121:
	.loc 1 384 10
	movq	-32(%rbp), %rsi
	movq	-24(%rbp), %rax
	movl	$1, %ecx
	movl	$1, %edx
	movq	%rax, %rdi
	call	cbb_add_child
.L118:
	.loc 1 385 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE147:
	.size	aws_lc_0_38_0_CBB_add_asn1, .-aws_lc_0_38_0_CBB_add_asn1
	.section	.text.aws_lc_0_38_0_CBB_add_bytes,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_bytes
	.type	aws_lc_0_38_0_CBB_add_bytes, @function
aws_lc_0_38_0_CBB_add_bytes:
.LFB148:
	.loc 1 387 62
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	movq	%rdx, -40(%rbp)
	.loc 1 389 8
	movq	-40(%rbp), %rdx
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_space@PLT
	.loc 1 389 6 discriminator 1
	testl	%eax, %eax
	jne	.L123
	.loc 1 390 12
	movl	$0, %eax
	jmp	.L125
.L123:
	.loc 1 392 3
	movq	-8(%rbp), %rax
	movq	-40(%rbp), %rdx
	movq	-32(%rbp), %rcx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 393 10
	movl	$1, %eax
.L125:
	.loc 1 394 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE148:
	.size	aws_lc_0_38_0_CBB_add_bytes, .-aws_lc_0_38_0_CBB_add_bytes
	.section	.text.aws_lc_0_38_0_CBB_add_zeros,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_zeros
	.type	aws_lc_0_38_0_CBB_add_zeros, @function
aws_lc_0_38_0_CBB_add_zeros:
.LFB149:
	.loc 1 396 41
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 398 8
	movq	-32(%rbp), %rdx
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_space@PLT
	.loc 1 398 6 discriminator 1
	testl	%eax, %eax
	jne	.L127
	.loc 1 399 12
	movl	$0, %eax
	jmp	.L129
.L127:
	.loc 1 401 3
	movq	-8(%rbp), %rax
	movq	-32(%rbp), %rdx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 402 10
	movl	$1, %eax
.L129:
	.loc 1 403 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE149:
	.size	aws_lc_0_38_0_CBB_add_zeros, .-aws_lc_0_38_0_CBB_add_zeros
	.section	.text.aws_lc_0_38_0_CBB_add_space,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_space
	.type	aws_lc_0_38_0_CBB_add_space, @function
aws_lc_0_38_0_CBB_add_space:
.LFB150:
	.loc 1 405 61
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 1 406 8
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 406 6 discriminator 1
	testl	%eax, %eax
	je	.L131
	.loc 1 407 8
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_get_base
	movq	%rax, %rcx
	.loc 1 407 8 is_stmt 0 discriminator 1
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rax
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	cbb_buffer_add
	.loc 1 406 23 is_stmt 1 discriminator 1
	testl	%eax, %eax
	jne	.L132
.L131:
	.loc 1 408 12
	movl	$0, %eax
	jmp	.L133
.L132:
	.loc 1 410 10
	movl	$1, %eax
.L133:
	.loc 1 411 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE150:
	.size	aws_lc_0_38_0_CBB_add_space, .-aws_lc_0_38_0_CBB_add_space
	.section	.text.aws_lc_0_38_0_CBB_reserve,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_reserve
	.type	aws_lc_0_38_0_CBB_reserve, @function
aws_lc_0_38_0_CBB_reserve:
.LFB151:
	.loc 1 413 59
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 1 414 8
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 414 6 discriminator 1
	testl	%eax, %eax
	je	.L135
	.loc 1 415 8
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_get_base
	movq	%rax, %rcx
	.loc 1 415 8 is_stmt 0 discriminator 1
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rax
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	cbb_buffer_reserve
	.loc 1 414 23 is_stmt 1 discriminator 1
	testl	%eax, %eax
	jne	.L136
.L135:
	.loc 1 416 12
	movl	$0, %eax
	jmp	.L137
.L136:
	.loc 1 418 10
	movl	$1, %eax
.L137:
	.loc 1 419 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE151:
	.size	aws_lc_0_38_0_CBB_reserve, .-aws_lc_0_38_0_CBB_reserve
	.section	.text.aws_lc_0_38_0_CBB_did_write,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_did_write
	.type	aws_lc_0_38_0_CBB_did_write, @function
aws_lc_0_38_0_CBB_did_write:
.LFB152:
	.loc 1 421 41
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 422 32
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_get_base
	movq	%rax, -8(%rbp)
	.loc 1 423 23
	movq	-8(%rbp), %rax
	movq	8(%rax), %rdx
	.loc 1 423 10
	movq	-32(%rbp), %rax
	addq	%rdx, %rax
	movq	%rax, -16(%rbp)
	.loc 1 424 10
	movq	-24(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 424 6
	testq	%rax, %rax
	jne	.L139
	.loc 1 425 20
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 424 26 discriminator 1
	cmpq	%rax, -16(%rbp)
	jb	.L139
	.loc 1 426 20
	movq	-8(%rbp), %rax
	movq	16(%rax), %rax
	.loc 1 425 26
	cmpq	-16(%rbp), %rax
	jnb	.L140
.L139:
	.loc 1 427 12
	movl	$0, %eax
	jmp	.L141
.L140:
	.loc 1 429 13
	movq	-8(%rbp), %rax
	movq	-16(%rbp), %rdx
	movq	%rdx, 8(%rax)
	.loc 1 430 10
	movl	$1, %eax
.L141:
	.loc 1 431 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE152:
	.size	aws_lc_0_38_0_CBB_did_write, .-aws_lc_0_38_0_CBB_did_write
	.section	.text.cbb_add_u,"ax",@progbits
	.type	cbb_add_u, @function
cbb_add_u:
.LFB153:
	.loc 1 433 60
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	movq	%rdx, -40(%rbp)
	.loc 1 435 8
	movq	-40(%rbp), %rdx
	leaq	-16(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_space@PLT
	.loc 1 435 6 discriminator 1
	testl	%eax, %eax
	jne	.L143
	.loc 1 436 12
	movl	$0, %eax
	jmp	.L148
.L143:
.LBB11:
	.loc 1 439 15
	movq	-40(%rbp), %rax
	subq	$1, %rax
	movq	%rax, -8(%rbp)
	.loc 1 439 3
	jmp	.L145
.L146:
	.loc 1 440 8
	movq	-16(%rbp), %rdx
	movq	-8(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 440 12
	movq	-32(%rbp), %rdx
	movb	%dl, (%rax)
	.loc 1 441 7
	shrq	$8, -32(%rbp)
	.loc 1 439 46 discriminator 3
	subq	$1, -8(%rbp)
.L145:
	.loc 1 439 34 discriminator 1
	movq	-8(%rbp), %rax
	cmpq	-40(%rbp), %rax
	jb	.L146
.LBE11:
	.loc 1 445 6
	cmpq	$0, -32(%rbp)
	je	.L147
	.loc 1 446 5
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_on_error
	.loc 1 447 12
	movl	$0, %eax
	jmp	.L148
.L147:
	.loc 1 450 10
	movl	$1, %eax
.L148:
	.loc 1 451 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE153:
	.size	cbb_add_u, .-cbb_add_u
	.section	.text.aws_lc_0_38_0_CBB_add_u8,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u8
	.type	aws_lc_0_38_0_CBB_add_u8, @function
aws_lc_0_38_0_CBB_add_u8:
.LFB154:
	.loc 1 453 41
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, %eax
	movb	%al, -12(%rbp)
	.loc 1 454 10
	movzbl	-12(%rbp), %ecx
	movq	-8(%rbp), %rax
	movl	$1, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbb_add_u
	.loc 1 455 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE154:
	.size	aws_lc_0_38_0_CBB_add_u8, .-aws_lc_0_38_0_CBB_add_u8
	.section	.text.aws_lc_0_38_0_CBB_add_u16,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u16
	.type	aws_lc_0_38_0_CBB_add_u16, @function
aws_lc_0_38_0_CBB_add_u16:
.LFB155:
	.loc 1 457 43
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, %eax
	movw	%ax, -12(%rbp)
	.loc 1 458 10
	movzwl	-12(%rbp), %ecx
	movq	-8(%rbp), %rax
	movl	$2, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbb_add_u
	.loc 1 459 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE155:
	.size	aws_lc_0_38_0_CBB_add_u16, .-aws_lc_0_38_0_CBB_add_u16
	.section	.text.aws_lc_0_38_0_CBB_add_u16le,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u16le
	.type	aws_lc_0_38_0_CBB_add_u16le, @function
aws_lc_0_38_0_CBB_add_u16le:
.LFB156:
	.loc 1 461 45
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, %eax
	movw	%ax, -12(%rbp)
	.loc 1 462 27
	movzwl	-12(%rbp), %eax
	movl	%eax, %edi
	call	CRYPTO_bswap2
	.loc 1 462 10 discriminator 1
	movzwl	%ax, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u16@PLT
	.loc 1 463 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE156:
	.size	aws_lc_0_38_0_CBB_add_u16le, .-aws_lc_0_38_0_CBB_add_u16le
	.section	.text.aws_lc_0_38_0_CBB_add_u24,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u24
	.type	aws_lc_0_38_0_CBB_add_u24, @function
aws_lc_0_38_0_CBB_add_u24:
.LFB157:
	.loc 1 465 43
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, -12(%rbp)
	.loc 1 466 10
	movl	-12(%rbp), %ecx
	movq	-8(%rbp), %rax
	movl	$3, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbb_add_u
	.loc 1 467 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE157:
	.size	aws_lc_0_38_0_CBB_add_u24, .-aws_lc_0_38_0_CBB_add_u24
	.section	.text.aws_lc_0_38_0_CBB_add_u32,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u32
	.type	aws_lc_0_38_0_CBB_add_u32, @function
aws_lc_0_38_0_CBB_add_u32:
.LFB158:
	.loc 1 469 43
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, -12(%rbp)
	.loc 1 470 10
	movl	-12(%rbp), %ecx
	movq	-8(%rbp), %rax
	movl	$4, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbb_add_u
	.loc 1 471 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE158:
	.size	aws_lc_0_38_0_CBB_add_u32, .-aws_lc_0_38_0_CBB_add_u32
	.section	.text.aws_lc_0_38_0_CBB_add_u32le,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u32le
	.type	aws_lc_0_38_0_CBB_add_u32le, @function
aws_lc_0_38_0_CBB_add_u32le:
.LFB159:
	.loc 1 473 45
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, -12(%rbp)
	.loc 1 474 10
	movl	-12(%rbp), %eax
	movl	%eax, %edi
	call	CRYPTO_bswap4
	movl	%eax, %edx
	.loc 1 474 10 is_stmt 0 discriminator 1
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u32@PLT
	.loc 1 475 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE159:
	.size	aws_lc_0_38_0_CBB_add_u32le, .-aws_lc_0_38_0_CBB_add_u32le
	.section	.text.aws_lc_0_38_0_CBB_add_u64,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u64
	.type	aws_lc_0_38_0_CBB_add_u64, @function
aws_lc_0_38_0_CBB_add_u64:
.LFB160:
	.loc 1 477 43
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 478 10
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$8, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbb_add_u
	.loc 1 479 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE160:
	.size	aws_lc_0_38_0_CBB_add_u64, .-aws_lc_0_38_0_CBB_add_u64
	.section	.text.aws_lc_0_38_0_CBB_add_u64le,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_u64le
	.type	aws_lc_0_38_0_CBB_add_u64le, @function
aws_lc_0_38_0_CBB_add_u64le:
.LFB161:
	.loc 1 481 45
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 482 10
	movq	-16(%rbp), %rax
	movq	%rax, %rdi
	call	CRYPTO_bswap8
	movq	%rax, %rdx
	.loc 1 482 10 is_stmt 0 discriminator 1
	movq	-8(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u64@PLT
	.loc 1 483 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE161:
	.size	aws_lc_0_38_0_CBB_add_u64le, .-aws_lc_0_38_0_CBB_add_u64le
	.section	.text.aws_lc_0_38_0_CBB_discard_child,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_discard_child
	.type	aws_lc_0_38_0_CBB_discard_child, @function
aws_lc_0_38_0_CBB_discard_child:
.LFB162:
	.loc 1 485 34
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	.loc 1 486 10
	movq	-24(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 486 6
	testq	%rax, %rax
	je	.L169
	.loc 1 490 32
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_get_base
	movq	%rax, -8(%rbp)
	.loc 1 491 3
	movq	-24(%rbp), %rax
	movq	(%rax), %rax
	movzbl	8(%rax), %eax
	testb	%al, %al
	jne	.L168
	.loc 1 491 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.1(%rip), %rax
	movq	%rax, %rcx
	movl	$491, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC2(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L168:
	.loc 1 492 18 is_stmt 1
	movq	-24(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 492 34
	movq	24(%rax), %rdx
	.loc 1 492 13
	movq	-8(%rbp), %rax
	movq	%rdx, 8(%rax)
	.loc 1 494 6
	movq	-24(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 494 28
	movq	$0, 16(%rax)
	.loc 1 495 14
	movq	-24(%rbp), %rax
	movq	$0, (%rax)
	jmp	.L165
.L169:
	.loc 1 487 5
	nop
.L165:
	.loc 1 496 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE162:
	.size	aws_lc_0_38_0_CBB_discard_child, .-aws_lc_0_38_0_CBB_discard_child
	.section	.text.aws_lc_0_38_0_CBB_add_asn1_uint64,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_asn1_uint64
	.type	aws_lc_0_38_0_CBB_add_asn1_uint64, @function
aws_lc_0_38_0_CBB_add_asn1_uint64:
.LFB163:
	.loc 1 498 51
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 499 10
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$2, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_asn1_uint64_with_tag@PLT
	.loc 1 500 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE163:
	.size	aws_lc_0_38_0_CBB_add_asn1_uint64, .-aws_lc_0_38_0_CBB_add_asn1_uint64
	.section	.text.aws_lc_0_38_0_CBB_add_asn1_uint64_with_tag,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_asn1_uint64_with_tag
	.type	aws_lc_0_38_0_CBB_add_asn1_uint64_with_tag, @function
aws_lc_0_38_0_CBB_add_asn1_uint64_with_tag:
.LFB164:
	.loc 1 502 78
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$112, %rsp
	movq	%rdi, -88(%rbp)
	movq	%rsi, -96(%rbp)
	movl	%edx, -100(%rbp)
	.loc 1 504 8
	movl	-100(%rbp), %edx
	leaq	-80(%rbp), %rcx
	movq	-88(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_asn1@PLT
	.loc 1 504 6 discriminator 1
	testl	%eax, %eax
	je	.L184
	.loc 1 508 7
	movl	$0, -4(%rbp)
.LBB12:
	.loc 1 509 15
	movq	$0, -16(%rbp)
	.loc 1 509 3
	jmp	.L175
.L180:
.LBB13:
	.loc 1 510 37
	movl	$7, %eax
	subq	-16(%rbp), %rax
	.loc 1 510 27
	sall	$3, %eax
	movq	-96(%rbp), %rdx
	shrx	%rax, %rdx, %rax
	.loc 1 510 13
	movb	%al, -17(%rbp)
	.loc 1 511 8
	cmpl	$0, -4(%rbp)
	jne	.L176
	.loc 1 512 10
	cmpb	$0, -17(%rbp)
	je	.L185
	.loc 1 518 11
	movzbl	-17(%rbp), %eax
	.loc 1 518 10
	testb	%al, %al
	jns	.L179
	.loc 1 518 29 discriminator 1
	leaq	-80(%rbp), %rax
	movl	$0, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 518 25 discriminator 1
	testl	%eax, %eax
	je	.L186
.L179:
	.loc 1 521 15
	movl	$1, -4(%rbp)
.L176:
	.loc 1 523 10
	movzbl	-17(%rbp), %edx
	leaq	-80(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 523 8 discriminator 1
	testl	%eax, %eax
	je	.L187
	jmp	.L178
.L185:
	.loc 1 514 9
	nop
.L178:
.LBE13:
	.loc 1 509 30 discriminator 2
	addq	$1, -16(%rbp)
.L175:
	.loc 1 509 24 discriminator 1
	cmpq	$7, -16(%rbp)
	jbe	.L180
.LBE12:
	.loc 1 529 6
	cmpl	$0, -4(%rbp)
	jne	.L181
	.loc 1 529 20 discriminator 1
	leaq	-80(%rbp), %rax
	movl	$0, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 529 16 discriminator 1
	testl	%eax, %eax
	je	.L188
.L181:
	.loc 1 533 10
	movq	-88(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	jmp	.L183
.L184:
	.loc 1 505 5
	nop
	jmp	.L174
.L186:
.LBB15:
.LBB14:
	.loc 1 519 9
	nop
	jmp	.L174
.L187:
	.loc 1 524 7
	nop
	jmp	.L174
.L188:
.LBE14:
.LBE15:
	.loc 1 530 5
	nop
.L174:
	.loc 1 536 3
	movq	-88(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_on_error
	.loc 1 537 10
	movl	$0, %eax
.L183:
	.loc 1 538 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE164:
	.size	aws_lc_0_38_0_CBB_add_asn1_uint64_with_tag, .-aws_lc_0_38_0_CBB_add_asn1_uint64_with_tag
	.section	.text.aws_lc_0_38_0_CBB_add_asn1_int64,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_asn1_int64
	.type	aws_lc_0_38_0_CBB_add_asn1_int64, @function
aws_lc_0_38_0_CBB_add_asn1_int64:
.LFB165:
	.loc 1 540 49
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 541 10
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$2, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_asn1_int64_with_tag@PLT
	.loc 1 542 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE165:
	.size	aws_lc_0_38_0_CBB_add_asn1_int64, .-aws_lc_0_38_0_CBB_add_asn1_int64
	.section	.text.aws_lc_0_38_0_CBB_add_asn1_int64_with_tag,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_asn1_int64_with_tag
	.type	aws_lc_0_38_0_CBB_add_asn1_int64_with_tag, @function
aws_lc_0_38_0_CBB_add_asn1_int64_with_tag:
.LFB166:
	.loc 1 544 76
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$96, %rsp
	movq	%rdi, -72(%rbp)
	movq	%rsi, -80(%rbp)
	movl	%edx, -84(%rbp)
	.loc 1 545 13
	movq	-80(%rbp), %rax
	.loc 1 545 6
	testq	%rax, %rax
	js	.L192
	.loc 1 546 12
	movq	-80(%rbp), %rax
	movq	%rax, %rcx
	movl	-84(%rbp), %edx
	movq	-72(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_asn1_uint64_with_tag@PLT
	jmp	.L202
.L192:
	.loc 1 550 3
	movq	-80(%rbp), %rax
	movq	%rax, -16(%rbp)
	.loc 1 558 7
	movl	$7, -4(%rbp)
	.loc 1 559 9
	jmp	.L194
.L196:
	.loc 1 560 10
	subl	$1, -4(%rbp)
.L194:
	.loc 1 559 20
	cmpl	$0, -4(%rbp)
	jle	.L195
	.loc 1 559 29 discriminator 1
	movl	-4(%rbp), %eax
	cltq
	movzbl	-16(%rbp,%rax), %eax
	.loc 1 559 20 discriminator 1
	cmpb	$-1, %al
	jne	.L195
	.loc 1 559 61 discriminator 2
	movl	-4(%rbp), %eax
	subl	$1, %eax
	.loc 1 559 54 discriminator 2
	cltq
	movzbl	-16(%rbp,%rax), %eax
	.loc 1 559 45 discriminator 2
	testb	%al, %al
	js	.L196
.L195:
	.loc 1 564 8
	movl	-84(%rbp), %edx
	leaq	-64(%rbp), %rcx
	movq	-72(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_asn1@PLT
	.loc 1 564 6 discriminator 1
	testl	%eax, %eax
	je	.L203
.LBB16:
	.loc 1 570 12
	movl	-4(%rbp), %eax
	movl	%eax, -8(%rbp)
	.loc 1 570 3
	jmp	.L199
.L201:
	.loc 1 572 34
	movl	-8(%rbp), %eax
	cltq
	movzbl	-16(%rbp,%rax), %eax
	.loc 1 572 10
	movzbl	%al, %edx
	leaq	-64(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 572 8 discriminator 1
	testl	%eax, %eax
	je	.L204
	.loc 1 570 32 discriminator 2
	subl	$1, -8(%rbp)
.L199:
	.loc 1 570 25 discriminator 1
	cmpl	$0, -8(%rbp)
	jns	.L201
.LBE16:
	.loc 1 576 10
	movq	-72(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	jmp	.L202
.L203:
	.loc 1 565 5
	nop
	jmp	.L198
.L204:
.LBB17:
	.loc 1 573 7
	nop
.L198:
.LBE17:
	.loc 1 579 3
	movq	-72(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_on_error
	.loc 1 580 10
	movl	$0, %eax
.L202:
	.loc 1 581 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE166:
	.size	aws_lc_0_38_0_CBB_add_asn1_int64_with_tag, .-aws_lc_0_38_0_CBB_add_asn1_int64_with_tag
	.section	.text.aws_lc_0_38_0_CBB_add_asn1_octet_string,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_asn1_octet_string
	.type	aws_lc_0_38_0_CBB_add_asn1_octet_string, @function
aws_lc_0_38_0_CBB_add_asn1_octet_string:
.LFB167:
	.loc 1 583 79
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$80, %rsp
	movq	%rdi, -56(%rbp)
	movq	%rsi, -64(%rbp)
	movq	%rdx, -72(%rbp)
	.loc 1 585 8
	leaq	-48(%rbp), %rcx
	movq	-56(%rbp), %rax
	movl	$4, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_asn1@PLT
	.loc 1 585 6 discriminator 1
	testl	%eax, %eax
	je	.L206
	.loc 1 586 8
	movq	-72(%rbp), %rdx
	movq	-64(%rbp), %rcx
	leaq	-48(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_bytes@PLT
	.loc 1 585 56 discriminator 1
	testl	%eax, %eax
	je	.L206
	.loc 1 587 8
	movq	-56(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 586 46
	testl	%eax, %eax
	jne	.L207
.L206:
	.loc 1 588 5
	movq	-56(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_on_error
	.loc 1 589 12
	movl	$0, %eax
	jmp	.L209
.L207:
	.loc 1 592 10
	movl	$1, %eax
.L209:
	.loc 1 593 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE167:
	.size	aws_lc_0_38_0_CBB_add_asn1_octet_string, .-aws_lc_0_38_0_CBB_add_asn1_octet_string
	.section	.text.aws_lc_0_38_0_CBB_add_asn1_bool,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_asn1_bool
	.type	aws_lc_0_38_0_CBB_add_asn1_bool, @function
aws_lc_0_38_0_CBB_add_asn1_bool:
.LFB168:
	.loc 1 595 44
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -56(%rbp)
	movl	%esi, -60(%rbp)
	.loc 1 597 8
	leaq	-48(%rbp), %rcx
	movq	-56(%rbp), %rax
	movl	$1, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_asn1@PLT
	.loc 1 597 6 discriminator 1
	testl	%eax, %eax
	je	.L211
	.loc 1 598 8
	cmpl	$0, -60(%rbp)
	je	.L212
	.loc 1 598 8 is_stmt 0 discriminator 1
	movl	$255, %edx
	jmp	.L213
.L212:
	.loc 1 598 8 discriminator 2
	movl	$0, %edx
.L213:
	.loc 1 598 8 discriminator 4
	leaq	-48(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 597 52 is_stmt 1
	testl	%eax, %eax
	je	.L211
	.loc 1 599 8
	movq	-56(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 598 50
	testl	%eax, %eax
	jne	.L214
.L211:
	.loc 1 600 5
	movq	-56(%rbp), %rax
	movq	%rax, %rdi
	call	cbb_on_error
	.loc 1 601 12
	movl	$0, %eax
	jmp	.L216
.L214:
	.loc 1 604 10
	movl	$1, %eax
.L216:
	.loc 1 605 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE168:
	.size	aws_lc_0_38_0_CBB_add_asn1_bool, .-aws_lc_0_38_0_CBB_add_asn1_bool
	.section	.text.parse_dotted_decimal,"ax",@progbits
	.type	parse_dotted_decimal, @function
parse_dotted_decimal:
.LFB169:
	.loc 1 611 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 612 8
	movq	-32(%rbp), %rdx
	movq	-24(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u64_decimal@PLT
	.loc 1 612 6 discriminator 1
	testl	%eax, %eax
	jne	.L218
	.loc 1 613 12
	movl	$0, %eax
	jmp	.L223
.L218:
	.loc 1 620 11
	leaq	-1(%rbp), %rdx
	movq	-24(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 620 33 discriminator 1
	testl	%eax, %eax
	je	.L220
	.loc 1 620 41 discriminator 2
	movzbl	-1(%rbp), %eax
	.loc 1 620 33 discriminator 2
	cmpb	$46, %al
	jne	.L221
	.loc 1 620 51 discriminator 3
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 620 48 discriminator 1
	testq	%rax, %rax
	je	.L221
.L220:
	.loc 1 620 33 discriminator 5
	movl	$1, %eax
	.loc 1 620 33 is_stmt 0
	jmp	.L223
.L221:
	.loc 1 620 33 discriminator 6
	movl	$0, %eax
.L223:
	.loc 1 621 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE169:
	.size	parse_dotted_decimal, .-parse_dotted_decimal
	.section	.text.aws_lc_0_38_0_CBB_add_asn1_oid_from_text,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_add_asn1_oid_from_text
	.type	aws_lc_0_38_0_CBB_add_asn1_oid_from_text, @function
aws_lc_0_38_0_CBB_add_asn1_oid_from_text:
.LFB170:
	.loc 1 623 72
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	movq	%rdx, -56(%rbp)
	.loc 1 624 8
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 624 6 discriminator 1
	testl	%eax, %eax
	jne	.L225
	.loc 1 625 12
	movl	$0, %eax
	jmp	.L235
.L225:
	.loc 1 629 3
	movq	-56(%rbp), %rdx
	movq	-48(%rbp), %rcx
	leaq	-16(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_init@PLT
	.loc 1 633 8
	leaq	-24(%rbp), %rdx
	leaq	-16(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	parse_dotted_decimal
	.loc 1 633 6 discriminator 1
	testl	%eax, %eax
	je	.L227
	.loc 1 634 8
	leaq	-32(%rbp), %rdx
	leaq	-16(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	parse_dotted_decimal
	.loc 1 633 39 discriminator 1
	testl	%eax, %eax
	jne	.L228
.L227:
	.loc 1 635 12
	movl	$0, %eax
	jmp	.L235
.L228:
	.loc 1 640 9
	movq	-24(%rbp), %rax
	.loc 1 640 6
	cmpq	$2, %rax
	ja	.L229
	.loc 1 641 10
	movq	-24(%rbp), %rax
	.loc 1 640 13 discriminator 1
	cmpq	$1, %rax
	ja	.L230
	.loc 1 641 19
	movq	-32(%rbp), %rax
	.loc 1 641 14
	cmpq	$39, %rax
	ja	.L229
.L230:
	.loc 1 642 9
	movq	-32(%rbp), %rax
	.loc 1 641 25 discriminator 1
	cmpq	$-81, %rax
	ja	.L229
	.loc 1 643 37
	movq	-24(%rbp), %rdx
	movq	%rdx, %rax
	salq	$2, %rax
	addq	%rdx, %rax
	salq	$3, %rax
	movq	%rax, %rdx
	.loc 1 643 8
	movq	-32(%rbp), %rax
	addq	%rax, %rdx
	movq	-40(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	add_base128_integer
	.loc 1 642 27
	testl	%eax, %eax
	jne	.L232
.L229:
	.loc 1 644 12
	movl	$0, %eax
	jmp	.L235
.L234:
	.loc 1 649 10
	leaq	-24(%rbp), %rdx
	leaq	-16(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	parse_dotted_decimal
	.loc 1 649 8 discriminator 1
	testl	%eax, %eax
	je	.L233
	.loc 1 650 10
	movq	-24(%rbp), %rdx
	movq	-40(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	add_base128_integer
	.loc 1 649 41 discriminator 1
	testl	%eax, %eax
	jne	.L232
.L233:
	.loc 1 651 14
	movl	$0, %eax
	jmp	.L235
.L232:
	.loc 1 648 10
	leaq	-16(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 648 24 discriminator 1
	testq	%rax, %rax
	jne	.L234
	.loc 1 655 10
	movl	$1, %eax
.L235:
	.loc 1 656 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE170:
	.size	aws_lc_0_38_0_CBB_add_asn1_oid_from_text, .-aws_lc_0_38_0_CBB_add_asn1_oid_from_text
	.section	.text.compare_set_of_element,"ax",@progbits
	.type	compare_set_of_element, @function
compare_set_of_element:
.LFB171:
	.loc 1 658 73
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%rbx
	subq	$72, %rsp
	.cfi_offset 3, -24
	movq	%rdi, -72(%rbp)
	movq	%rsi, -80(%rbp)
	.loc 1 661 14
	movq	-72(%rbp), %rax
	movq	%rax, -24(%rbp)
	.loc 1 661 26
	movq	-80(%rbp), %rax
	movq	%rax, -32(%rbp)
	.loc 1 662 18
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, -40(%rbp)
	.loc 1 662 38 discriminator 1
	movq	-32(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, -48(%rbp)
	.loc 1 663 10
	movq	-48(%rbp), %rdx
	movq	-40(%rbp), %rax
	cmpq	%rax, %rdx
	cmovbe	%rdx, %rax
	movq	%rax, -56(%rbp)
	.loc 1 664 41
	movq	-32(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	movq	%rax, %rbx
	.loc 1 664 28 discriminator 1
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	movq	%rax, %rcx
	.loc 1 664 13 discriminator 2
	movq	-56(%rbp), %rax
	movq	%rax, %rdx
	movq	%rbx, %rsi
	movq	%rcx, %rdi
	call	OPENSSL_memcmp
	movl	%eax, -60(%rbp)
	.loc 1 665 6
	cmpl	$0, -60(%rbp)
	je	.L237
	.loc 1 666 12
	movl	-60(%rbp), %eax
	jmp	.L238
.L237:
	.loc 1 668 6
	movq	-40(%rbp), %rax
	cmpq	-48(%rbp), %rax
	jne	.L239
	.loc 1 669 12
	movl	$0, %eax
	jmp	.L238
.L239:
	.loc 1 673 29
	movq	-40(%rbp), %rax
	cmpq	-48(%rbp), %rax
	jnb	.L240
	.loc 1 673 29 is_stmt 0 discriminator 1
	movl	$-1, %eax
	.loc 1 673 29
	jmp	.L238
.L240:
	.loc 1 673 29 discriminator 2
	movl	$1, %eax
.L238:
	.loc 1 674 1 is_stmt 1
	movq	-8(%rbp), %rbx
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE171:
	.size	compare_set_of_element, .-compare_set_of_element
	.section	.rodata
.LC8:
	.string	"offset == buf_len"
	.section	.text.aws_lc_0_38_0_CBB_flush_asn1_set_of,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_flush_asn1_set_of
	.type	aws_lc_0_38_0_CBB_flush_asn1_set_of, @function
aws_lc_0_38_0_CBB_flush_asn1_set_of:
.LFB172:
	.loc 1 676 37
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%rbx
	subq	$120, %rsp
	.cfi_offset 3, -24
	movq	%rdi, -120(%rbp)
	.loc 1 677 8
	movq	-120(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 677 6 discriminator 1
	testl	%eax, %eax
	jne	.L243
	.loc 1 678 12
	movl	$0, %eax
	jmp	.L258
.L243:
	.loc 1 682 10
	movq	$0, -24(%rbp)
	.loc 1 683 3
	movq	-120(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_len@PLT
	movq	%rax, %rbx
	.loc 1 683 3 is_stmt 0 discriminator 1
	movq	-120(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_data@PLT
	movq	%rax, %rcx
	.loc 1 683 3 discriminator 2
	leaq	-112(%rbp), %rax
	movq	%rbx, %rdx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_init@PLT
	.loc 1 684 9 is_stmt 1
	jmp	.L245
.L247:
	.loc 1 685 10
	leaq	-112(%rbp), %rax
	movl	$0, %ecx
	movl	$0, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_any_asn1_element@PLT
	.loc 1 685 8 discriminator 1
	testl	%eax, %eax
	jne	.L246
	.loc 1 686 7
	movl	$686, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$66, %edx
	movl	$0, %esi
	movl	$14, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 687 14
	movl	$0, %eax
	jmp	.L258
.L246:
	.loc 1 689 17
	addq	$1, -24(%rbp)
.L245:
	.loc 1 684 10
	leaq	-112(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 684 24 discriminator 1
	testq	%rax, %rax
	jne	.L247
	.loc 1 692 6
	cmpq	$1, -24(%rbp)
	ja	.L248
	.loc 1 693 12
	movl	$1, %eax
	jmp	.L258
.L248:
	.loc 1 698 7
	movl	$0, -28(%rbp)
	.loc 1 699 20
	movq	-120(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_len@PLT
	movq	%rax, -64(%rbp)
	.loc 1 700 33
	movq	-120(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_data@PLT
	movq	%rax, %rdx
	.loc 1 700 18 discriminator 1
	movq	-64(%rbp), %rax
	movq	%rax, %rsi
	movq	%rdx, %rdi
	call	aws_lc_0_38_0_OPENSSL_memdup@PLT
	movq	%rax, -72(%rbp)
	.loc 1 701 19
	movq	-24(%rbp), %rax
	movl	$16, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_calloc@PLT
	movq	%rax, -80(%rbp)
	.loc 1 702 6
	cmpq	$0, -72(%rbp)
	je	.L259
	.loc 1 702 19 discriminator 1
	cmpq	$0, -80(%rbp)
	je	.L259
	.loc 1 705 3
	movq	-64(%rbp), %rdx
	movq	-72(%rbp), %rcx
	leaq	-112(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_init@PLT
.LBB18:
	.loc 1 706 15
	movq	$0, -40(%rbp)
	.loc 1 706 3
	jmp	.L252
.L254:
	.loc 1 707 50
	movq	-40(%rbp), %rax
	salq	$4, %rax
	movq	%rax, %rdx
	.loc 1 707 10
	movq	-80(%rbp), %rax
	leaq	(%rdx,%rax), %rsi
	leaq	-112(%rbp), %rax
	movl	$0, %ecx
	movl	$0, %edx
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_any_asn1_element@PLT
	.loc 1 707 8 discriminator 1
	testl	%eax, %eax
	je	.L260
	.loc 1 706 41 discriminator 2
	addq	$1, -40(%rbp)
.L252:
	.loc 1 706 24 discriminator 1
	movq	-40(%rbp), %rax
	cmpq	-24(%rbp), %rax
	jb	.L254
.LBE18:
	.loc 1 711 3
	movq	-24(%rbp), %rsi
	movq	-80(%rbp), %rax
	leaq	compare_set_of_element(%rip), %rdx
	movq	%rdx, %rcx
	movl	$16, %edx
	movq	%rax, %rdi
	call	qsort@PLT
	.loc 1 714 29
	movq	-120(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_data@PLT
	movq	%rax, -88(%rbp)
	.loc 1 715 10
	movq	$0, -48(%rbp)
.LBB19:
	.loc 1 716 15
	movq	$0, -56(%rbp)
	.loc 1 716 3
	jmp	.L255
.L256:
	.loc 1 717 75
	movq	-56(%rbp), %rax
	salq	$4, %rax
	movq	%rax, %rdx
	.loc 1 717 66
	movq	-80(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 717 5
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, %rbx
	.loc 1 717 52 discriminator 1
	movq	-56(%rbp), %rax
	salq	$4, %rax
	movq	%rax, %rdx
	.loc 1 717 43 discriminator 1
	movq	-80(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 717 34 discriminator 1
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	movq	%rax, %rcx
	.loc 1 717 24 discriminator 2
	movq	-88(%rbp), %rdx
	movq	-48(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 717 5 discriminator 2
	movq	%rbx, %rdx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 718 32
	movq	-56(%rbp), %rax
	salq	$4, %rax
	movq	%rax, %rdx
	.loc 1 718 23
	movq	-80(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 718 15
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 718 12 discriminator 1
	addq	%rax, -48(%rbp)
	.loc 1 716 41 discriminator 3
	addq	$1, -56(%rbp)
.L255:
	.loc 1 716 24 discriminator 1
	movq	-56(%rbp), %rax
	cmpq	-24(%rbp), %rax
	jb	.L256
.LBE19:
	.loc 1 720 3
	movq	-48(%rbp), %rax
	cmpq	-64(%rbp), %rax
	je	.L257
	.loc 1 720 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$720, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC8(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L257:
	.loc 1 722 7 is_stmt 1
	movl	$1, -28(%rbp)
	jmp	.L251
.L259:
	.loc 1 703 5
	nop
	jmp	.L251
.L260:
.LBB20:
	.loc 1 708 7
	nop
.L251:
.LBE20:
	.loc 1 725 3
	movq	-72(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_free@PLT
	.loc 1 726 3
	movq	-80(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_free@PLT
	.loc 1 727 10
	movl	-28(%rbp), %eax
.L258:
	.loc 1 728 1
	movq	-8(%rbp), %rbx
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE172:
	.size	aws_lc_0_38_0_CBB_flush_asn1_set_of, .-aws_lc_0_38_0_CBB_flush_asn1_set_of
	.section	.rodata.__PRETTY_FUNCTION__.6,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.6, @object
	.size	__PRETTY_FUNCTION__.6, 26
__PRETTY_FUNCTION__.6:
	.string	"aws_lc_0_38_0_CBB_cleanup"
	.section	.rodata.__PRETTY_FUNCTION__.5,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.5, @object
	.size	__PRETTY_FUNCTION__.5, 24
__PRETTY_FUNCTION__.5:
	.string	"aws_lc_0_38_0_CBB_flush"
	.section	.rodata.__PRETTY_FUNCTION__.4,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.4, @object
	.size	__PRETTY_FUNCTION__.4, 23
__PRETTY_FUNCTION__.4:
	.string	"aws_lc_0_38_0_CBB_data"
	.section	.rodata.__PRETTY_FUNCTION__.3,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.3, @object
	.size	__PRETTY_FUNCTION__.3, 22
__PRETTY_FUNCTION__.3:
	.string	"aws_lc_0_38_0_CBB_len"
	.section	.rodata.__PRETTY_FUNCTION__.2,"a"
	.align 8
	.type	__PRETTY_FUNCTION__.2, @object
	.size	__PRETTY_FUNCTION__.2, 14
__PRETTY_FUNCTION__.2:
	.string	"cbb_add_child"
	.section	.rodata.__PRETTY_FUNCTION__.1,"a"
	.align 32
	.type	__PRETTY_FUNCTION__.1, @object
	.size	__PRETTY_FUNCTION__.1, 32
__PRETTY_FUNCTION__.1:
	.string	"aws_lc_0_38_0_CBB_discard_child"
	.section	.rodata.__PRETTY_FUNCTION__.0,"a"
	.align 32
	.type	__PRETTY_FUNCTION__.0, @object
	.size	__PRETTY_FUNCTION__.0, 36
__PRETTY_FUNCTION__.0:
	.string	"aws_lc_0_38_0_CBB_flush_asn1_set_of"
	.text
.Letext0:
	.file 3 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 4 "/usr/include/bits/types.h"
	.file 5 "/usr/include/bits/stdint-intn.h"
	.file 6 "/usr/include/bits/stdint-uintn.h"
	.file 7 "/usr/include/stdlib.h"
	.file 8 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.file 9 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/bytestring.h"
	.file 10 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/mem.h"
	.file 11 "/usr/include/string.h"
	.file 12 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/err.h"
	.file 13 "/usr/include/assert.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x18c7
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF144
	.byte	0xc
	.long	.LASF145
	.long	.LASF146
	.long	.Ldebug_ranges0+0x150
	.quad	0
	.long	.Ldebug_line0
	.uleb128 0x2
	.byte	0x8
	.byte	0x5
	.long	.LASF0
	.uleb128 0x3
	.long	.LASF8
	.byte	0x3
	.byte	0xe5
	.byte	0x17
	.long	0x3c
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF1
	.uleb128 0x4
	.byte	0x4
	.byte	0x5
	.string	"int"
	.uleb128 0x2
	.byte	0x8
	.byte	0x5
	.long	.LASF2
	.uleb128 0x2
	.byte	0x10
	.byte	0x4
	.long	.LASF3
	.uleb128 0x2
	.byte	0x1
	.byte	0x8
	.long	.LASF4
	.uleb128 0x2
	.byte	0x2
	.byte	0x7
	.long	.LASF5
	.uleb128 0x2
	.byte	0x4
	.byte	0x7
	.long	.LASF6
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF7
	.uleb128 0x3
	.long	.LASF9
	.byte	0x4
	.byte	0x26
	.byte	0x17
	.long	0x58
	.uleb128 0x2
	.byte	0x2
	.byte	0x5
	.long	.LASF10
	.uleb128 0x3
	.long	.LASF11
	.byte	0x4
	.byte	0x28
	.byte	0x1c
	.long	0x5f
	.uleb128 0x3
	.long	.LASF12
	.byte	0x4
	.byte	0x2a
	.byte	0x16
	.long	0x66
	.uleb128 0x3
	.long	.LASF13
	.byte	0x4
	.byte	0x2c
	.byte	0x19
	.long	0x29
	.uleb128 0x3
	.long	.LASF14
	.byte	0x4
	.byte	0x2d
	.byte	0x1b
	.long	0x3c
	.uleb128 0x5
	.byte	0x8
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF15
	.uleb128 0x6
	.long	0xb9
	.uleb128 0x3
	.long	.LASF16
	.byte	0x5
	.byte	0x1b
	.byte	0x13
	.long	0x9f
	.uleb128 0x3
	.long	.LASF17
	.byte	0x6
	.byte	0x18
	.byte	0x13
	.long	0x74
	.uleb128 0x6
	.long	0xd1
	.uleb128 0x3
	.long	.LASF18
	.byte	0x6
	.byte	0x19
	.byte	0x14
	.long	0x87
	.uleb128 0x3
	.long	.LASF19
	.byte	0x6
	.byte	0x1a
	.byte	0x14
	.long	0x93
	.uleb128 0x3
	.long	.LASF20
	.byte	0x6
	.byte	0x1b
	.byte	0x14
	.long	0xab
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF21
	.uleb128 0x7
	.long	.LASF22
	.byte	0x7
	.value	0x3b4
	.byte	0xf
	.long	0x11a
	.uleb128 0x8
	.byte	0x8
	.long	0x120
	.uleb128 0x9
	.long	0x43
	.long	0x134
	.uleb128 0xa
	.long	0x134
	.uleb128 0xa
	.long	0x134
	.byte	0
	.uleb128 0x8
	.byte	0x8
	.long	0x13a
	.uleb128 0xb
	.uleb128 0x7
	.long	.LASF23
	.byte	0x8
	.value	0x158
	.byte	0x12
	.long	0xee
	.uleb128 0xc
	.string	"CBB"
	.byte	0x8
	.value	0x194
	.byte	0x17
	.long	0x15a
	.uleb128 0x6
	.long	0x148
	.uleb128 0xd
	.long	.LASF26
	.byte	0x30
	.byte	0x9
	.value	0x1be
	.byte	0x8
	.long	0x191
	.uleb128 0xe
	.long	.LASF24
	.byte	0x9
	.value	0x1c0
	.byte	0x8
	.long	0x2ad
	.byte	0
	.uleb128 0xe
	.long	.LASF25
	.byte	0x9
	.value	0x1c3
	.byte	0x8
	.long	0xb9
	.byte	0x8
	.uleb128 0xf
	.string	"u"
	.byte	0x9
	.value	0x1c7
	.byte	0x5
	.long	0x288
	.byte	0x10
	.byte	0
	.uleb128 0xc
	.string	"CBS"
	.byte	0x8
	.value	0x195
	.byte	0x17
	.long	0x1a3
	.uleb128 0x6
	.long	0x191
	.uleb128 0x10
	.long	.LASF27
	.byte	0x10
	.byte	0x9
	.byte	0x28
	.byte	0x8
	.long	0x1cb
	.uleb128 0x11
	.long	.LASF28
	.byte	0x9
	.byte	0x29
	.byte	0x12
	.long	0x1d1
	.byte	0
	.uleb128 0x12
	.string	"len"
	.byte	0x9
	.byte	0x2a
	.byte	0xa
	.long	0x30
	.byte	0x8
	.byte	0
	.uleb128 0x8
	.byte	0x8
	.long	0xc0
	.uleb128 0x8
	.byte	0x8
	.long	0xdd
	.uleb128 0xd
	.long	.LASF29
	.byte	0x20
	.byte	0x9
	.value	0x1a4
	.byte	0x8
	.long	0x232
	.uleb128 0xf
	.string	"buf"
	.byte	0x9
	.value	0x1a5
	.byte	0xc
	.long	0x232
	.byte	0
	.uleb128 0xf
	.string	"len"
	.byte	0x9
	.value	0x1a7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0xf
	.string	"cap"
	.byte	0x9
	.value	0x1a9
	.byte	0xa
	.long	0x30
	.byte	0x10
	.uleb128 0x13
	.long	.LASF30
	.byte	0x9
	.value	0x1ac
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x1f
	.byte	0x18
	.uleb128 0x13
	.long	.LASF31
	.byte	0x9
	.value	0x1af
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x1e
	.byte	0x18
	.byte	0
	.uleb128 0x8
	.byte	0x8
	.long	0xd1
	.uleb128 0xd
	.long	.LASF32
	.byte	0x18
	.byte	0x9
	.value	0x1b2
	.byte	0x8
	.long	0x282
	.uleb128 0xe
	.long	.LASF33
	.byte	0x9
	.value	0x1b4
	.byte	0x19
	.long	0x282
	.byte	0
	.uleb128 0xe
	.long	.LASF34
	.byte	0x9
	.value	0x1b7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0xe
	.long	.LASF35
	.byte	0x9
	.value	0x1ba
	.byte	0xb
	.long	0xd1
	.byte	0x10
	.uleb128 0x13
	.long	.LASF36
	.byte	0x9
	.value	0x1bb
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x17
	.byte	0x10
	.byte	0
	.uleb128 0x8
	.byte	0x8
	.long	0x1d7
	.uleb128 0x14
	.byte	0x20
	.byte	0x9
	.value	0x1c4
	.byte	0x3
	.long	0x2ad
	.uleb128 0x15
	.long	.LASF33
	.byte	0x9
	.value	0x1c5
	.byte	0x1a
	.long	0x1d7
	.uleb128 0x15
	.long	.LASF24
	.byte	0x9
	.value	0x1c6
	.byte	0x19
	.long	0x238
	.byte	0
	.uleb128 0x8
	.byte	0x8
	.long	0x148
	.uleb128 0x2
	.byte	0x8
	.byte	0x4
	.long	.LASF37
	.uleb128 0x2
	.byte	0x10
	.byte	0x5
	.long	.LASF38
	.uleb128 0x2
	.byte	0x10
	.byte	0x7
	.long	.LASF39
	.uleb128 0x2
	.byte	0x1
	.byte	0x2
	.long	.LASF40
	.uleb128 0x16
	.long	.LASF46
	.byte	0x7
	.value	0x3ca
	.byte	0xd
	.long	0x2f1
	.uleb128 0xa
	.long	0xb7
	.uleb128 0xa
	.long	0x30
	.uleb128 0xa
	.long	0x30
	.uleb128 0xa
	.long	0x10d
	.byte	0
	.uleb128 0x17
	.long	.LASF41
	.byte	0xa
	.byte	0x5c
	.byte	0x16
	.long	0xb7
	.long	0x30c
	.uleb128 0xa
	.long	0x30
	.uleb128 0xa
	.long	0x30
	.byte	0
	.uleb128 0x17
	.long	.LASF42
	.byte	0xa
	.byte	0xce
	.byte	0x16
	.long	0xb7
	.long	0x327
	.uleb128 0xa
	.long	0x134
	.uleb128 0xa
	.long	0x30
	.byte	0
	.uleb128 0x18
	.long	.LASF43
	.byte	0x9
	.value	0x10c
	.byte	0x14
	.long	0x43
	.long	0x34d
	.uleb128 0xa
	.long	0x34d
	.uleb128 0xa
	.long	0x34d
	.uleb128 0xa
	.long	0x353
	.uleb128 0xa
	.long	0x359
	.byte	0
	.uleb128 0x8
	.byte	0x8
	.long	0x191
	.uleb128 0x8
	.byte	0x8
	.long	0x13b
	.uleb128 0x8
	.byte	0x8
	.long	0x30
	.uleb128 0x17
	.long	.LASF44
	.byte	0xb
	.byte	0x40
	.byte	0xc
	.long	0x43
	.long	0x37f
	.uleb128 0xa
	.long	0x134
	.uleb128 0xa
	.long	0x134
	.uleb128 0xa
	.long	0x3c
	.byte	0
	.uleb128 0x17
	.long	.LASF45
	.byte	0x9
	.byte	0x44
	.byte	0x1f
	.long	0x1d1
	.long	0x395
	.uleb128 0xa
	.long	0x395
	.byte	0
	.uleb128 0x8
	.byte	0x8
	.long	0x19e
	.uleb128 0x19
	.long	.LASF47
	.byte	0x9
	.byte	0x3d
	.byte	0x15
	.long	0x3b7
	.uleb128 0xa
	.long	0x34d
	.uleb128 0xa
	.long	0x1d1
	.uleb128 0xa
	.long	0x30
	.byte	0
	.uleb128 0x17
	.long	.LASF48
	.byte	0x9
	.byte	0x47
	.byte	0x17
	.long	0x30
	.long	0x3cd
	.uleb128 0xa
	.long	0x395
	.byte	0
	.uleb128 0x17
	.long	.LASF49
	.byte	0x9
	.byte	0x65
	.byte	0x14
	.long	0x43
	.long	0x3e8
	.uleb128 0xa
	.long	0x34d
	.uleb128 0xa
	.long	0x232
	.byte	0
	.uleb128 0x17
	.long	.LASF50
	.byte	0x9
	.byte	0xa8
	.byte	0x14
	.long	0x43
	.long	0x403
	.uleb128 0xa
	.long	0x34d
	.uleb128 0xa
	.long	0x403
	.byte	0
	.uleb128 0x8
	.byte	0x8
	.long	0xfa
	.uleb128 0x17
	.long	.LASF51
	.byte	0xb
	.byte	0x2b
	.byte	0xe
	.long	0xb7
	.long	0x429
	.uleb128 0xa
	.long	0xb7
	.uleb128 0xa
	.long	0x134
	.uleb128 0xa
	.long	0x3c
	.byte	0
	.uleb128 0x17
	.long	.LASF52
	.byte	0xb
	.byte	0x2f
	.byte	0xe
	.long	0xb7
	.long	0x449
	.uleb128 0xa
	.long	0xb7
	.uleb128 0xa
	.long	0x134
	.uleb128 0xa
	.long	0x3c
	.byte	0
	.uleb128 0x17
	.long	.LASF53
	.byte	0xa
	.byte	0x63
	.byte	0x16
	.long	0xb7
	.long	0x464
	.uleb128 0xa
	.long	0xb7
	.uleb128 0xa
	.long	0x30
	.byte	0
	.uleb128 0x16
	.long	.LASF54
	.byte	0xc
	.value	0x1d0
	.byte	0x15
	.long	0x48b
	.uleb128 0xa
	.long	0x43
	.uleb128 0xa
	.long	0x43
	.uleb128 0xa
	.long	0x43
	.uleb128 0xa
	.long	0x1cb
	.uleb128 0xa
	.long	0x66
	.byte	0
	.uleb128 0x19
	.long	.LASF55
	.byte	0xa
	.byte	0x69
	.byte	0x15
	.long	0x49d
	.uleb128 0xa
	.long	0xb7
	.byte	0
	.uleb128 0x1a
	.long	.LASF56
	.byte	0xd
	.byte	0x43
	.byte	0xd
	.long	0x4be
	.uleb128 0xa
	.long	0x1cb
	.uleb128 0xa
	.long	0x1cb
	.uleb128 0xa
	.long	0x66
	.uleb128 0xa
	.long	0x1cb
	.byte	0
	.uleb128 0x17
	.long	.LASF57
	.byte	0xa
	.byte	0x53
	.byte	0x16
	.long	0xb7
	.long	0x4d4
	.uleb128 0xa
	.long	0x30
	.byte	0
	.uleb128 0x17
	.long	.LASF58
	.byte	0xb
	.byte	0x3d
	.byte	0xe
	.long	0xb7
	.long	0x4f4
	.uleb128 0xa
	.long	0xb7
	.uleb128 0xa
	.long	0x43
	.uleb128 0xa
	.long	0x3c
	.byte	0
	.uleb128 0x1b
	.long	.LASF67
	.byte	0x1
	.value	0x2a4
	.byte	0x5
	.long	0x43
	.quad	.LFB172
	.quad	.LFE172-.LFB172
	.uleb128 0x1
	.byte	0x9c
	.long	0x60b
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x2a4
	.byte	0x20
	.long	0x2ad
	.uleb128 0x3
	.byte	0x91
	.sleb128 -136
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x2a9
	.byte	0x7
	.long	0x191
	.uleb128 0x3
	.byte	0x91
	.sleb128 -128
	.uleb128 0x1e
	.long	.LASF59
	.byte	0x1
	.value	0x2aa
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1d
	.string	"ret"
	.byte	0x1
	.value	0x2ba
	.byte	0x7
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -44
	.uleb128 0x1e
	.long	.LASF60
	.byte	0x1
	.value	0x2bb
	.byte	0xa
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x1d
	.string	"buf"
	.byte	0x1
	.value	0x2bc
	.byte	0xc
	.long	0x232
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x1e
	.long	.LASF61
	.byte	0x1
	.value	0x2bd
	.byte	0x8
	.long	0x34d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x1f
	.string	"err"
	.byte	0x1
	.value	0x2d4
	.byte	0x1
	.quad	.L251
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x2ca
	.byte	0xc
	.long	0x232
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x1e
	.long	.LASF34
	.byte	0x1
	.value	0x2cb
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x20
	.long	.LASF84
	.long	0x61b
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.0
	.uleb128 0x21
	.long	.Ldebug_ranges0+0x120
	.long	0x5e9
	.uleb128 0x1d
	.string	"i"
	.byte	0x1
	.value	0x2c2
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.byte	0
	.uleb128 0x22
	.quad	.LBB19
	.quad	.LBE19-.LBB19
	.uleb128 0x1d
	.string	"i"
	.byte	0x1
	.value	0x2cc
	.byte	0xf
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.byte	0
	.byte	0
	.uleb128 0x23
	.long	0xc0
	.long	0x61b
	.uleb128 0x24
	.long	0x3c
	.byte	0x23
	.byte	0
	.uleb128 0x6
	.long	0x60b
	.uleb128 0x25
	.long	.LASF70
	.byte	0x1
	.value	0x292
	.byte	0xc
	.long	0x43
	.quad	.LFB171
	.quad	.LFE171-.LFB171
	.uleb128 0x1
	.byte	0x9c
	.long	0x6c4
	.uleb128 0x26
	.long	.LASF62
	.byte	0x1
	.value	0x292
	.byte	0x2f
	.long	0x134
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x26
	.long	.LASF63
	.byte	0x1
	.value	0x292
	.byte	0x42
	.long	0x134
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x1d
	.string	"a"
	.byte	0x1
	.value	0x295
	.byte	0xe
	.long	0x395
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1d
	.string	"b"
	.byte	0x1
	.value	0x295
	.byte	0x1a
	.long	0x395
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1e
	.long	.LASF64
	.byte	0x1
	.value	0x296
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1e
	.long	.LASF65
	.byte	0x1
	.value	0x296
	.byte	0x1e
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1e
	.long	.LASF66
	.byte	0x1
	.value	0x297
	.byte	0xa
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x1d
	.string	"ret"
	.byte	0x1
	.value	0x298
	.byte	0x7
	.long	0x43
	.uleb128 0x3
	.byte	0x91
	.sleb128 -76
	.byte	0
	.uleb128 0x1b
	.long	.LASF68
	.byte	0x1
	.value	0x26f
	.byte	0x5
	.long	0x43
	.quad	.LFB170
	.quad	.LFE170-.LFB170
	.uleb128 0x1
	.byte	0x9c
	.long	0x745
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x26f
	.byte	0x25
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x26
	.long	.LASF69
	.byte	0x1
	.value	0x26f
	.byte	0x36
	.long	0x1cb
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1c
	.string	"len"
	.byte	0x1
	.value	0x26f
	.byte	0x43
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x274
	.byte	0x7
	.long	0x191
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1d
	.string	"a"
	.byte	0x1
	.value	0x278
	.byte	0xc
	.long	0xfa
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1d
	.string	"b"
	.byte	0x1
	.value	0x278
	.byte	0xf
	.long	0xfa
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.byte	0
	.uleb128 0x25
	.long	.LASF71
	.byte	0x1
	.value	0x263
	.byte	0xc
	.long	0x43
	.quad	.LFB169
	.quad	.LFE169-.LFB169
	.uleb128 0x1
	.byte	0x9c
	.long	0x799
	.uleb128 0x1c
	.string	"cbs"
	.byte	0x1
	.value	0x263
	.byte	0x26
	.long	0x34d
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1c
	.string	"out"
	.byte	0x1
	.value	0x263
	.byte	0x35
	.long	0x403
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1d
	.string	"dot"
	.byte	0x1
	.value	0x26b
	.byte	0xb
	.long	0xd1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -17
	.byte	0
	.uleb128 0x1b
	.long	.LASF72
	.byte	0x1
	.value	0x253
	.byte	0x5
	.long	0x43
	.quad	.LFB168
	.quad	.LFE168-.LFB168
	.uleb128 0x1
	.byte	0x9c
	.long	0x7ef
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x253
	.byte	0x1c
	.long	0x2ad
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x253
	.byte	0x25
	.long	0x43
	.uleb128 0x3
	.byte	0x91
	.sleb128 -76
	.uleb128 0x1e
	.long	.LASF24
	.byte	0x1
	.value	0x254
	.byte	0x7
	.long	0x148
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.byte	0
	.uleb128 0x1b
	.long	.LASF74
	.byte	0x1
	.value	0x247
	.byte	0x5
	.long	0x43
	.quad	.LFB167
	.quad	.LFE167-.LFB167
	.uleb128 0x1
	.byte	0x9c
	.long	0x856
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x247
	.byte	0x24
	.long	0x2ad
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x26
	.long	.LASF28
	.byte	0x1
	.value	0x247
	.byte	0x38
	.long	0x1d1
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x26
	.long	.LASF75
	.byte	0x1
	.value	0x247
	.byte	0x45
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x1e
	.long	.LASF24
	.byte	0x1
	.value	0x248
	.byte	0x7
	.long	0x148
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.byte	0
	.uleb128 0x1b
	.long	.LASF76
	.byte	0x1
	.value	0x220
	.byte	0x5
	.long	0x43
	.quad	.LFB166
	.quad	.LFE166-.LFB166
	.uleb128 0x1
	.byte	0x9c
	.long	0x903
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x220
	.byte	0x26
	.long	0x2ad
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x220
	.byte	0x33
	.long	0xc5
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x1c
	.string	"tag"
	.byte	0x1
	.value	0x220
	.byte	0x47
	.long	0x13b
	.uleb128 0x3
	.byte	0x91
	.sleb128 -100
	.uleb128 0x1e
	.long	.LASF77
	.byte	0x1
	.value	0x225
	.byte	0xb
	.long	0x903
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1e
	.long	.LASF78
	.byte	0x1
	.value	0x22e
	.byte	0x7
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x1e
	.long	.LASF24
	.byte	0x1
	.value	0x233
	.byte	0x7
	.long	0x148
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x1f
	.string	"err"
	.byte	0x1
	.value	0x242
	.byte	0x1
	.quad	.L198
	.uleb128 0x27
	.long	.Ldebug_ranges0+0xf0
	.uleb128 0x1d
	.string	"i"
	.byte	0x1
	.value	0x23a
	.byte	0xc
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.byte	0
	.uleb128 0x23
	.long	0xd1
	.long	0x913
	.uleb128 0x24
	.long	0x3c
	.byte	0x7
	.byte	0
	.uleb128 0x1b
	.long	.LASF79
	.byte	0x1
	.value	0x21c
	.byte	0x5
	.long	0x43
	.quad	.LFB165
	.quad	.LFE165-.LFB165
	.uleb128 0x1
	.byte	0x9c
	.long	0x957
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x21c
	.byte	0x1d
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x21c
	.byte	0x2a
	.long	0xc5
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x1b
	.long	.LASF80
	.byte	0x1
	.value	0x1f6
	.byte	0x5
	.long	0x43
	.quad	.LFB164
	.quad	.LFE164-.LFB164
	.uleb128 0x1
	.byte	0x9c
	.long	0xa0a
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1f6
	.byte	0x27
	.long	0x2ad
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x1f6
	.byte	0x35
	.long	0xfa
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x1c
	.string	"tag"
	.byte	0x1
	.value	0x1f6
	.byte	0x49
	.long	0x13b
	.uleb128 0x3
	.byte	0x91
	.sleb128 -116
	.uleb128 0x1e
	.long	.LASF24
	.byte	0x1
	.value	0x1f7
	.byte	0x7
	.long	0x148
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x1f
	.string	"err"
	.byte	0x1
	.value	0x217
	.byte	0x1
	.quad	.L174
	.uleb128 0x1e
	.long	.LASF81
	.byte	0x1
	.value	0x1fc
	.byte	0x7
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x27
	.long	.Ldebug_ranges0+0x90
	.uleb128 0x1d
	.string	"i"
	.byte	0x1
	.value	0x1fd
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x27
	.long	.Ldebug_ranges0+0xc0
	.uleb128 0x1e
	.long	.LASF82
	.byte	0x1
	.value	0x1fe
	.byte	0xd
	.long	0xd1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -33
	.byte	0
	.byte	0
	.byte	0
	.uleb128 0x1b
	.long	.LASF83
	.byte	0x1
	.value	0x1f2
	.byte	0x5
	.long	0x43
	.quad	.LFB163
	.quad	.LFE163-.LFB163
	.uleb128 0x1
	.byte	0x9c
	.long	0xa4e
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1f2
	.byte	0x1e
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x1f2
	.byte	0x2c
	.long	0xfa
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x28
	.long	.LASF129
	.byte	0x1
	.value	0x1e5
	.byte	0x6
	.quad	.LFB162
	.quad	.LFE162-.LFB162
	.uleb128 0x1
	.byte	0x9c
	.long	0xaa1
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1e5
	.byte	0x1d
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1e
	.long	.LASF33
	.byte	0x1
	.value	0x1ea
	.byte	0x19
	.long	0x282
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x20
	.long	.LASF84
	.long	0xab1
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.1
	.byte	0
	.uleb128 0x23
	.long	0xc0
	.long	0xab1
	.uleb128 0x24
	.long	0x3c
	.byte	0x1f
	.byte	0
	.uleb128 0x6
	.long	0xaa1
	.uleb128 0x1b
	.long	.LASF85
	.byte	0x1
	.value	0x1e1
	.byte	0x5
	.long	0x43
	.quad	.LFB161
	.quad	.LFE161-.LFB161
	.uleb128 0x1
	.byte	0x9c
	.long	0xafa
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1e1
	.byte	0x18
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x1e1
	.byte	0x26
	.long	0xfa
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x1b
	.long	.LASF86
	.byte	0x1
	.value	0x1dd
	.byte	0x5
	.long	0x43
	.quad	.LFB160
	.quad	.LFE160-.LFB160
	.uleb128 0x1
	.byte	0x9c
	.long	0xb3e
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1dd
	.byte	0x16
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x1dd
	.byte	0x24
	.long	0xfa
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x1b
	.long	.LASF87
	.byte	0x1
	.value	0x1d9
	.byte	0x5
	.long	0x43
	.quad	.LFB159
	.quad	.LFE159-.LFB159
	.uleb128 0x1
	.byte	0x9c
	.long	0xb82
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1d9
	.byte	0x18
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x1d9
	.byte	0x26
	.long	0xee
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x1b
	.long	.LASF88
	.byte	0x1
	.value	0x1d5
	.byte	0x5
	.long	0x43
	.quad	.LFB158
	.quad	.LFE158-.LFB158
	.uleb128 0x1
	.byte	0x9c
	.long	0xbc6
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1d5
	.byte	0x16
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x1d5
	.byte	0x24
	.long	0xee
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x1b
	.long	.LASF89
	.byte	0x1
	.value	0x1d1
	.byte	0x5
	.long	0x43
	.quad	.LFB157
	.quad	.LFE157-.LFB157
	.uleb128 0x1
	.byte	0x9c
	.long	0xc0a
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1d1
	.byte	0x16
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x1d1
	.byte	0x24
	.long	0xee
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x1b
	.long	.LASF90
	.byte	0x1
	.value	0x1cd
	.byte	0x5
	.long	0x43
	.quad	.LFB156
	.quad	.LFE156-.LFB156
	.uleb128 0x1
	.byte	0x9c
	.long	0xc4e
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1cd
	.byte	0x18
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x1cd
	.byte	0x26
	.long	0xe2
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x1b
	.long	.LASF91
	.byte	0x1
	.value	0x1c9
	.byte	0x5
	.long	0x43
	.quad	.LFB155
	.quad	.LFE155-.LFB155
	.uleb128 0x1
	.byte	0x9c
	.long	0xc92
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1c9
	.byte	0x16
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x1c9
	.byte	0x24
	.long	0xe2
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x1b
	.long	.LASF92
	.byte	0x1
	.value	0x1c5
	.byte	0x5
	.long	0x43
	.quad	.LFB154
	.quad	.LFE154-.LFB154
	.uleb128 0x1
	.byte	0x9c
	.long	0xcd6
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1c5
	.byte	0x15
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF73
	.byte	0x1
	.value	0x1c5
	.byte	0x22
	.long	0xd1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x25
	.long	.LASF93
	.byte	0x1
	.value	0x1b1
	.byte	0xc
	.long	0x43
	.quad	.LFB153
	.quad	.LFE153-.LFB153
	.uleb128 0x1
	.byte	0x9c
	.long	0xd58
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1b1
	.byte	0x1b
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1c
	.string	"v"
	.byte	0x1
	.value	0x1b1
	.byte	0x29
	.long	0xfa
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x26
	.long	.LASF94
	.byte	0x1
	.value	0x1b1
	.byte	0x33
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"buf"
	.byte	0x1
	.value	0x1b2
	.byte	0xc
	.long	0x232
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x22
	.quad	.LBB11
	.quad	.LBE11-.LBB11
	.uleb128 0x1d
	.string	"i"
	.byte	0x1
	.value	0x1b7
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.byte	0
	.uleb128 0x1b
	.long	.LASF95
	.byte	0x1
	.value	0x1a5
	.byte	0x5
	.long	0x43
	.quad	.LFB152
	.quad	.LFE152-.LFB152
	.uleb128 0x1
	.byte	0x9c
	.long	0xdbc
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x1a5
	.byte	0x18
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1c
	.string	"len"
	.byte	0x1
	.value	0x1a5
	.byte	0x24
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1e
	.long	.LASF33
	.byte	0x1
	.value	0x1a6
	.byte	0x19
	.long	0x282
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1e
	.long	.LASF96
	.byte	0x1
	.value	0x1a7
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x1b
	.long	.LASF97
	.byte	0x1
	.value	0x19d
	.byte	0x5
	.long	0x43
	.quad	.LFB151
	.quad	.LFE151-.LFB151
	.uleb128 0x1
	.byte	0x9c
	.long	0xe10
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x19d
	.byte	0x16
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF98
	.byte	0x1
	.value	0x19d
	.byte	0x25
	.long	0xe10
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1c
	.string	"len"
	.byte	0x1
	.value	0x19d
	.byte	0x36
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x8
	.byte	0x8
	.long	0x232
	.uleb128 0x1b
	.long	.LASF99
	.byte	0x1
	.value	0x195
	.byte	0x5
	.long	0x43
	.quad	.LFB150
	.quad	.LFE150-.LFB150
	.uleb128 0x1
	.byte	0x9c
	.long	0xe6a
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x195
	.byte	0x18
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF98
	.byte	0x1
	.value	0x195
	.byte	0x27
	.long	0xe10
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1c
	.string	"len"
	.byte	0x1
	.value	0x195
	.byte	0x38
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x1b
	.long	.LASF100
	.byte	0x1
	.value	0x18c
	.byte	0x5
	.long	0x43
	.quad	.LFB149
	.quad	.LFE149-.LFB149
	.uleb128 0x1
	.byte	0x9c
	.long	0xebe
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x18c
	.byte	0x18
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1c
	.string	"len"
	.byte	0x1
	.value	0x18c
	.byte	0x24
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x18d
	.byte	0xc
	.long	0x232
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x1b
	.long	.LASF101
	.byte	0x1
	.value	0x183
	.byte	0x5
	.long	0x43
	.quad	.LFB148
	.quad	.LFE148-.LFB148
	.uleb128 0x1
	.byte	0x9c
	.long	0xf22
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x183
	.byte	0x18
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x26
	.long	.LASF28
	.byte	0x1
	.value	0x183
	.byte	0x2c
	.long	0x1d1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1c
	.string	"len"
	.byte	0x1
	.value	0x183
	.byte	0x39
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x184
	.byte	0xc
	.long	0x232
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x1b
	.long	.LASF102
	.byte	0x1
	.value	0x16d
	.byte	0x5
	.long	0x43
	.quad	.LFB147
	.quad	.LFE147-.LFB147
	.uleb128 0x1
	.byte	0x9c
	.long	0xf96
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x16d
	.byte	0x17
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x26
	.long	.LASF103
	.byte	0x1
	.value	0x16d
	.byte	0x21
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1c
	.string	"tag"
	.byte	0x1
	.value	0x16d
	.byte	0x3c
	.long	0x13b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -52
	.uleb128 0x1e
	.long	.LASF104
	.byte	0x1
	.value	0x173
	.byte	0xb
	.long	0xd1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -17
	.uleb128 0x1e
	.long	.LASF105
	.byte	0x1
	.value	0x174
	.byte	0x10
	.long	0x13b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x25
	.long	.LASF106
	.byte	0x1
	.value	0x156
	.byte	0xc
	.long	0x43
	.quad	.LFB146
	.quad	.LFE146-.LFB146
	.uleb128 0x1
	.byte	0x9c
	.long	0x103a
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x156
	.byte	0x25
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1c
	.string	"v"
	.byte	0x1
	.value	0x156
	.byte	0x33
	.long	0xfa
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1e
	.long	.LASF94
	.byte	0x1
	.value	0x157
	.byte	0xc
	.long	0x66
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x1e
	.long	.LASF107
	.byte	0x1
	.value	0x158
	.byte	0xc
	.long	0xfa
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x22
	.quad	.LBB9
	.quad	.LBE9-.LBB9
	.uleb128 0x1d
	.string	"i"
	.byte	0x1
	.value	0x160
	.byte	0x11
	.long	0x66
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.uleb128 0x22
	.quad	.LBB10
	.quad	.LBE10-.LBB10
	.uleb128 0x1e
	.long	.LASF82
	.byte	0x1
	.value	0x161
	.byte	0xd
	.long	0xd1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -37
	.byte	0
	.byte	0
	.byte	0
	.uleb128 0x1b
	.long	.LASF108
	.byte	0x1
	.value	0x14f
	.byte	0x5
	.long	0x43
	.quad	.LFB145
	.quad	.LFE145-.LFB145
	.uleb128 0x1
	.byte	0x9c
	.long	0x107e
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x14f
	.byte	0x26
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF103
	.byte	0x1
	.value	0x14f
	.byte	0x30
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x1b
	.long	.LASF109
	.byte	0x1
	.value	0x14b
	.byte	0x5
	.long	0x43
	.quad	.LFB144
	.quad	.LFE144-.LFB144
	.uleb128 0x1
	.byte	0x9c
	.long	0x10c2
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x14b
	.byte	0x26
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF103
	.byte	0x1
	.value	0x14b
	.byte	0x30
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x1b
	.long	.LASF110
	.byte	0x1
	.value	0x147
	.byte	0x5
	.long	0x43
	.quad	.LFB143
	.quad	.LFE143-.LFB143
	.uleb128 0x1
	.byte	0x9c
	.long	0x1106
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x147
	.byte	0x25
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF103
	.byte	0x1
	.value	0x147
	.byte	0x2f
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x25
	.long	.LASF111
	.byte	0x1
	.value	0x13e
	.byte	0xc
	.long	0x43
	.quad	.LFB142
	.quad	.LFE142-.LFB142
	.uleb128 0x1
	.byte	0x9c
	.long	0x115a
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x13e
	.byte	0x29
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x26
	.long	.LASF103
	.byte	0x1
	.value	0x13e
	.byte	0x33
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x26
	.long	.LASF94
	.byte	0x1
	.value	0x13f
	.byte	0x2c
	.long	0xd1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.byte	0
	.uleb128 0x25
	.long	.LASF112
	.byte	0x1
	.value	0x126
	.byte	0xc
	.long	0x43
	.quad	.LFB141
	.quad	.LFE141-.LFB141
	.uleb128 0x1
	.byte	0x9c
	.long	0x1203
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x126
	.byte	0x1f
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x26
	.long	.LASF113
	.byte	0x1
	.value	0x126
	.byte	0x29
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x26
	.long	.LASF94
	.byte	0x1
	.value	0x126
	.byte	0x3c
	.long	0xd1
	.uleb128 0x3
	.byte	0x91
	.sleb128 -68
	.uleb128 0x26
	.long	.LASF114
	.byte	0x1
	.value	0x127
	.byte	0x1e
	.long	0x43
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x20
	.long	.LASF84
	.long	0x1213
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.2
	.uleb128 0x1e
	.long	.LASF33
	.byte	0x1
	.value	0x12a
	.byte	0x19
	.long	0x282
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1e
	.long	.LASF34
	.byte	0x1
	.value	0x12b
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1e
	.long	.LASF115
	.byte	0x1
	.value	0x12e
	.byte	0xc
	.long	0x232
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x23
	.long	0xc0
	.long	0x1213
	.uleb128 0x24
	.long	0x3c
	.byte	0xd
	.byte	0
	.uleb128 0x6
	.long	0x1203
	.uleb128 0x1b
	.long	.LASF116
	.byte	0x1
	.value	0x11b
	.byte	0x8
	.long	0x30
	.quad	.LFB140
	.quad	.LFE140-.LFB140
	.uleb128 0x1
	.byte	0x9c
	.long	0x125f
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x11b
	.byte	0x1b
	.long	0x125f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x20
	.long	.LASF84
	.long	0x1275
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.3
	.byte	0
	.uleb128 0x8
	.byte	0x8
	.long	0x155
	.uleb128 0x23
	.long	0xc0
	.long	0x1275
	.uleb128 0x24
	.long	0x3c
	.byte	0x15
	.byte	0
	.uleb128 0x6
	.long	0x1265
	.uleb128 0x1b
	.long	.LASF117
	.byte	0x1
	.value	0x112
	.byte	0x10
	.long	0x1d1
	.quad	.LFB139
	.quad	.LFE139-.LFB139
	.uleb128 0x1
	.byte	0x9c
	.long	0x12c1
	.uleb128 0x1c
	.string	"cbb"
	.byte	0x1
	.value	0x112
	.byte	0x24
	.long	0x125f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x20
	.long	.LASF84
	.long	0x12d1
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.4
	.byte	0
	.uleb128 0x23
	.long	0xc0
	.long	0x12d1
	.uleb128 0x24
	.long	0x3c
	.byte	0x16
	.byte	0
	.uleb128 0x6
	.long	0x12c1
	.uleb128 0x29
	.long	.LASF118
	.byte	0x1
	.byte	0xb8
	.byte	0x5
	.long	0x43
	.quad	.LFB138
	.quad	.LFE138-.LFB138
	.uleb128 0x1
	.byte	0x9c
	.long	0x13c6
	.uleb128 0x2a
	.string	"cbb"
	.byte	0x1
	.byte	0xb8
	.byte	0x14
	.long	0x2ad
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x2b
	.long	.LASF33
	.byte	0x1
	.byte	0xbc
	.byte	0x19
	.long	0x282
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x20
	.long	.LASF84
	.long	0x13d6
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.5
	.uleb128 0x2b
	.long	.LASF24
	.byte	0x1
	.byte	0xc7
	.byte	0x18
	.long	0x13db
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x2b
	.long	.LASF119
	.byte	0x1
	.byte	0xc9
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1f
	.string	"err"
	.byte	0x1
	.value	0x10d
	.byte	0x1
	.quad	.L67
	.uleb128 0x2c
	.string	"len"
	.byte	0x1
	.byte	0xd1
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x21
	.long	.Ldebug_ranges0+0x30
	.long	0x13a6
	.uleb128 0x2b
	.long	.LASF94
	.byte	0x1
	.byte	0xd7
	.byte	0xd
	.long	0xd1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -25
	.uleb128 0x2b
	.long	.LASF120
	.byte	0x1
	.byte	0xd8
	.byte	0xd
	.long	0xd1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -26
	.uleb128 0x27
	.long	.Ldebug_ranges0+0x60
	.uleb128 0x2b
	.long	.LASF121
	.byte	0x1
	.byte	0xf4
	.byte	0xe
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.byte	0
	.byte	0
	.uleb128 0x22
	.quad	.LBB7
	.quad	.LBE7-.LBB7
	.uleb128 0x2c
	.string	"i"
	.byte	0x1
	.byte	0xff
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.byte	0
	.uleb128 0x23
	.long	0xc0
	.long	0x13d6
	.uleb128 0x24
	.long	0x3c
	.byte	0x17
	.byte	0
	.uleb128 0x6
	.long	0x13c6
	.uleb128 0x8
	.byte	0x8
	.long	0x238
	.uleb128 0x2d
	.long	.LASF134
	.byte	0x1
	.byte	0x9e
	.byte	0xd
	.quad	.LFB137
	.quad	.LFE137-.LFB137
	.uleb128 0x1
	.byte	0x9c
	.long	0x140f
	.uleb128 0x2a
	.string	"cbb"
	.byte	0x1
	.byte	0x9e
	.byte	0x1f
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x2e
	.long	.LASF122
	.byte	0x1
	.byte	0x97
	.byte	0x1e
	.long	0x282
	.quad	.LFB136
	.quad	.LFE136-.LFB136
	.uleb128 0x1
	.byte	0x9c
	.long	0x1441
	.uleb128 0x2a
	.string	"cbb"
	.byte	0x1
	.byte	0x97
	.byte	0x30
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x29
	.long	.LASF123
	.byte	0x1
	.byte	0x7d
	.byte	0x5
	.long	0x43
	.quad	.LFB135
	.quad	.LFE135-.LFB135
	.uleb128 0x1
	.byte	0x9c
	.long	0x1491
	.uleb128 0x2a
	.string	"cbb"
	.byte	0x1
	.byte	0x7d
	.byte	0x15
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2f
	.long	.LASF98
	.byte	0x1
	.byte	0x7d
	.byte	0x24
	.long	0xe10
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x2f
	.long	.LASF124
	.byte	0x1
	.byte	0x7d
	.byte	0x36
	.long	0x359
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x30
	.long	.LASF125
	.byte	0x1
	.byte	0x73
	.byte	0xc
	.long	0x43
	.quad	.LFB134
	.quad	.LFE134-.LFB134
	.uleb128 0x1
	.byte	0x9c
	.long	0x14e1
	.uleb128 0x2f
	.long	.LASF33
	.byte	0x1
	.byte	0x73
	.byte	0x31
	.long	0x282
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2a
	.string	"out"
	.byte	0x1
	.byte	0x73
	.byte	0x41
	.long	0xe10
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x2a
	.string	"len"
	.byte	0x1
	.byte	0x74
	.byte	0x22
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x30
	.long	.LASF126
	.byte	0x1
	.byte	0x48
	.byte	0xc
	.long	0x43
	.quad	.LFB133
	.quad	.LFE133-.LFB133
	.uleb128 0x1
	.byte	0x9c
	.long	0x1575
	.uleb128 0x2f
	.long	.LASF33
	.byte	0x1
	.byte	0x48
	.byte	0x35
	.long	0x282
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x2a
	.string	"out"
	.byte	0x1
	.byte	0x48
	.byte	0x45
	.long	0xe10
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x2a
	.string	"len"
	.byte	0x1
	.byte	0x49
	.byte	0x26
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x2b
	.long	.LASF96
	.byte	0x1
	.byte	0x4e
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x31
	.string	"err"
	.byte	0x1
	.byte	0x6e
	.byte	0x1
	.quad	.L35
	.uleb128 0x27
	.long	.Ldebug_ranges0+0
	.uleb128 0x2b
	.long	.LASF127
	.byte	0x1
	.byte	0x5b
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2b
	.long	.LASF128
	.byte	0x1
	.byte	0x5f
	.byte	0xe
	.long	0x232
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.byte	0
	.uleb128 0x32
	.long	.LASF130
	.byte	0x1
	.byte	0x3b
	.byte	0x6
	.quad	.LFB132
	.quad	.LFE132-.LFB132
	.uleb128 0x1
	.byte	0x9c
	.long	0x15b6
	.uleb128 0x2a
	.string	"cbb"
	.byte	0x1
	.byte	0x3b
	.byte	0x17
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x20
	.long	.LASF84
	.long	0x15c6
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.6
	.byte	0
	.uleb128 0x23
	.long	0xc0
	.long	0x15c6
	.uleb128 0x24
	.long	0x3c
	.byte	0x19
	.byte	0
	.uleb128 0x6
	.long	0x15b6
	.uleb128 0x29
	.long	.LASF131
	.byte	0x1
	.byte	0x35
	.byte	0x5
	.long	0x43
	.quad	.LFB131
	.quad	.LFE131-.LFB131
	.uleb128 0x1
	.byte	0x9c
	.long	0x161b
	.uleb128 0x2a
	.string	"cbb"
	.byte	0x1
	.byte	0x35
	.byte	0x19
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2a
	.string	"buf"
	.byte	0x1
	.byte	0x35
	.byte	0x27
	.long	0x232
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x2a
	.string	"len"
	.byte	0x1
	.byte	0x35
	.byte	0x33
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x29
	.long	.LASF132
	.byte	0x1
	.byte	0x29
	.byte	0x5
	.long	0x43
	.quad	.LFB130
	.quad	.LFE130-.LFB130
	.uleb128 0x1
	.byte	0x9c
	.long	0x166b
	.uleb128 0x2a
	.string	"cbb"
	.byte	0x1
	.byte	0x29
	.byte	0x13
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x2f
	.long	.LASF133
	.byte	0x1
	.byte	0x29
	.byte	0x1f
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x2c
	.string	"buf"
	.byte	0x1
	.byte	0x2c
	.byte	0xc
	.long	0x232
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x33
	.long	.LASF135
	.byte	0x1
	.byte	0x1f
	.byte	0xd
	.quad	.LFB129
	.quad	.LFE129-.LFB129
	.uleb128 0x1
	.byte	0x9c
	.long	0x16c6
	.uleb128 0x2a
	.string	"cbb"
	.byte	0x1
	.byte	0x1f
	.byte	0x1b
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2a
	.string	"buf"
	.byte	0x1
	.byte	0x1f
	.byte	0x29
	.long	0x232
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x2a
	.string	"cap"
	.byte	0x1
	.byte	0x1f
	.byte	0x35
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x2f
	.long	.LASF30
	.byte	0x1
	.byte	0x1f
	.byte	0x3e
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -44
	.byte	0
	.uleb128 0x32
	.long	.LASF136
	.byte	0x1
	.byte	0x1b
	.byte	0x6
	.quad	.LFB128
	.quad	.LFE128-.LFB128
	.uleb128 0x1
	.byte	0x9c
	.long	0x16f4
	.uleb128 0x2a
	.string	"cbb"
	.byte	0x1
	.byte	0x1b
	.byte	0x14
	.long	0x2ad
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x25
	.long	.LASF137
	.byte	0x2
	.value	0x3cc
	.byte	0x15
	.long	0xb7
	.quad	.LFB96
	.quad	.LFE96-.LFB96
	.uleb128 0x1
	.byte	0x9c
	.long	0x1744
	.uleb128 0x1c
	.string	"dst"
	.byte	0x2
	.value	0x3cc
	.byte	0x2a
	.long	0xb7
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1c
	.string	"c"
	.byte	0x2
	.value	0x3cc
	.byte	0x33
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.uleb128 0x1c
	.string	"n"
	.byte	0x2
	.value	0x3cc
	.byte	0x3d
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x25
	.long	.LASF138
	.byte	0x2
	.value	0x3c4
	.byte	0x15
	.long	0xb7
	.quad	.LFB95
	.quad	.LFE95-.LFB95
	.uleb128 0x1
	.byte	0x9c
	.long	0x1796
	.uleb128 0x1c
	.string	"dst"
	.byte	0x2
	.value	0x3c4
	.byte	0x2b
	.long	0xb7
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1c
	.string	"src"
	.byte	0x2
	.value	0x3c4
	.byte	0x3c
	.long	0x134
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1c
	.string	"n"
	.byte	0x2
	.value	0x3c4
	.byte	0x48
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x25
	.long	.LASF139
	.byte	0x2
	.value	0x3bc
	.byte	0x15
	.long	0xb7
	.quad	.LFB94
	.quad	.LFE94-.LFB94
	.uleb128 0x1
	.byte	0x9c
	.long	0x17e8
	.uleb128 0x1c
	.string	"dst"
	.byte	0x2
	.value	0x3bc
	.byte	0x2a
	.long	0xb7
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1c
	.string	"src"
	.byte	0x2
	.value	0x3bc
	.byte	0x3b
	.long	0x134
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1c
	.string	"n"
	.byte	0x2
	.value	0x3bc
	.byte	0x47
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x25
	.long	.LASF140
	.byte	0x2
	.value	0x3b4
	.byte	0x13
	.long	0x43
	.quad	.LFB93
	.quad	.LFE93-.LFB93
	.uleb128 0x1
	.byte	0x9c
	.long	0x1838
	.uleb128 0x1c
	.string	"s1"
	.byte	0x2
	.value	0x3b4
	.byte	0x2e
	.long	0x134
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1c
	.string	"s2"
	.byte	0x2
	.value	0x3b4
	.byte	0x3e
	.long	0x134
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1c
	.string	"n"
	.byte	0x2
	.value	0x3b4
	.byte	0x49
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x34
	.long	.LASF141
	.byte	0x2
	.value	0x356
	.byte	0x18
	.long	0xfa
	.quad	.LFB90
	.quad	.LFE90-.LFB90
	.uleb128 0x1
	.byte	0x9c
	.long	0x186a
	.uleb128 0x1c
	.string	"x"
	.byte	0x2
	.value	0x356
	.byte	0x2f
	.long	0xfa
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x34
	.long	.LASF142
	.byte	0x2
	.value	0x352
	.byte	0x18
	.long	0xee
	.quad	.LFB89
	.quad	.LFE89-.LFB89
	.uleb128 0x1
	.byte	0x9c
	.long	0x189c
	.uleb128 0x1c
	.string	"x"
	.byte	0x2
	.value	0x352
	.byte	0x2f
	.long	0xee
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.uleb128 0x35
	.long	.LASF143
	.byte	0x2
	.value	0x34e
	.byte	0x18
	.long	0xe2
	.quad	.LFB88
	.quad	.LFE88-.LFB88
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x1c
	.string	"x"
	.byte	0x2
	.value	0x34e
	.byte	0x2f
	.long	0xe2
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.byte	0
	.section	.debug_abbrev,"",@progbits
.Ldebug_abbrev0:
	.uleb128 0x1
	.uleb128 0x11
	.byte	0x1
	.uleb128 0x25
	.uleb128 0xe
	.uleb128 0x13
	.uleb128 0xb
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x1b
	.uleb128 0xe
	.uleb128 0x55
	.uleb128 0x17
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x10
	.uleb128 0x17
	.byte	0
	.byte	0
	.uleb128 0x2
	.uleb128 0x24
	.byte	0
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x3e
	.uleb128 0xb
	.uleb128 0x3
	.uleb128 0xe
	.byte	0
	.byte	0
	.uleb128 0x3
	.uleb128 0x16
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x4
	.uleb128 0x24
	.byte	0
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x3e
	.uleb128 0xb
	.uleb128 0x3
	.uleb128 0x8
	.byte	0
	.byte	0
	.uleb128 0x5
	.uleb128 0xf
	.byte	0
	.uleb128 0xb
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x6
	.uleb128 0x26
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x7
	.uleb128 0x16
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x8
	.uleb128 0xf
	.byte	0
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x9
	.uleb128 0x15
	.byte	0x1
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0xa
	.uleb128 0x5
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0xb
	.uleb128 0x26
	.byte	0
	.byte	0
	.byte	0
	.uleb128 0xc
	.uleb128 0x16
	.byte	0
	.uleb128 0x3
	.uleb128 0x8
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0xd
	.uleb128 0x13
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0xe
	.uleb128 0xd
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x38
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0xf
	.uleb128 0xd
	.byte	0
	.uleb128 0x3
	.uleb128 0x8
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x38
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x10
	.uleb128 0x13
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x11
	.uleb128 0xd
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x38
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x12
	.uleb128 0xd
	.byte	0
	.uleb128 0x3
	.uleb128 0x8
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x38
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x13
	.uleb128 0xd
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0xd
	.uleb128 0xb
	.uleb128 0xc
	.uleb128 0xb
	.uleb128 0x38
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x14
	.uleb128 0x17
	.byte	0x1
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x15
	.uleb128 0xd
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x16
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3f
	.uleb128 0x19
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x3c
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x17
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3f
	.uleb128 0x19
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x3c
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x18
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3f
	.uleb128 0x19
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x3c
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x19
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3f
	.uleb128 0x19
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x3c
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x1a
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3f
	.uleb128 0x19
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x87
	.uleb128 0x19
	.uleb128 0x3c
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x1b
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3f
	.uleb128 0x19
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2116
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x1c
	.uleb128 0x5
	.byte	0
	.uleb128 0x3
	.uleb128 0x8
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x1d
	.uleb128 0x34
	.byte	0
	.uleb128 0x3
	.uleb128 0x8
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x1e
	.uleb128 0x34
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x1f
	.uleb128 0xa
	.byte	0
	.uleb128 0x3
	.uleb128 0x8
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x11
	.uleb128 0x1
	.byte	0
	.byte	0
	.uleb128 0x20
	.uleb128 0x34
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x34
	.uleb128 0x19
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x21
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x55
	.uleb128 0x17
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x22
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.byte	0
	.byte	0
	.uleb128 0x23
	.uleb128 0x1
	.byte	0x1
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x24
	.uleb128 0x21
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2f
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x25
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2116
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x26
	.uleb128 0x5
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x27
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x55
	.uleb128 0x17
	.byte	0
	.byte	0
	.uleb128 0x28
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3f
	.uleb128 0x19
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2116
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x29
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3f
	.uleb128 0x19
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2116
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x2a
	.uleb128 0x5
	.byte	0
	.uleb128 0x3
	.uleb128 0x8
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x2b
	.uleb128 0x34
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x2c
	.uleb128 0x34
	.byte	0
	.uleb128 0x3
	.uleb128 0x8
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x2d
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2116
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x2e
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2117
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x2f
	.uleb128 0x5
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x30
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2116
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x31
	.uleb128 0xa
	.byte	0
	.uleb128 0x3
	.uleb128 0x8
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x11
	.uleb128 0x1
	.byte	0
	.byte	0
	.uleb128 0x32
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3f
	.uleb128 0x19
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2116
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x33
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0xb
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2117
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x34
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2117
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x35
	.uleb128 0x2e
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3a
	.uleb128 0xb
	.uleb128 0x3b
	.uleb128 0x5
	.uleb128 0x39
	.uleb128 0xb
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x40
	.uleb128 0x18
	.uleb128 0x2117
	.uleb128 0x19
	.byte	0
	.byte	0
	.byte	0
	.section	.debug_aranges,"",@progbits
	.long	0x35c
	.value	0x2
	.long	.Ldebug_info0
	.byte	0x8
	.byte	0
	.value	0
	.value	0
	.quad	.LFB88
	.quad	.LFE88-.LFB88
	.quad	.LFB89
	.quad	.LFE89-.LFB89
	.quad	.LFB90
	.quad	.LFE90-.LFB90
	.quad	.LFB93
	.quad	.LFE93-.LFB93
	.quad	.LFB94
	.quad	.LFE94-.LFB94
	.quad	.LFB95
	.quad	.LFE95-.LFB95
	.quad	.LFB96
	.quad	.LFE96-.LFB96
	.quad	.LFB128
	.quad	.LFE128-.LFB128
	.quad	.LFB129
	.quad	.LFE129-.LFB129
	.quad	.LFB130
	.quad	.LFE130-.LFB130
	.quad	.LFB131
	.quad	.LFE131-.LFB131
	.quad	.LFB132
	.quad	.LFE132-.LFB132
	.quad	.LFB133
	.quad	.LFE133-.LFB133
	.quad	.LFB134
	.quad	.LFE134-.LFB134
	.quad	.LFB135
	.quad	.LFE135-.LFB135
	.quad	.LFB136
	.quad	.LFE136-.LFB136
	.quad	.LFB137
	.quad	.LFE137-.LFB137
	.quad	.LFB138
	.quad	.LFE138-.LFB138
	.quad	.LFB139
	.quad	.LFE139-.LFB139
	.quad	.LFB140
	.quad	.LFE140-.LFB140
	.quad	.LFB141
	.quad	.LFE141-.LFB141
	.quad	.LFB142
	.quad	.LFE142-.LFB142
	.quad	.LFB143
	.quad	.LFE143-.LFB143
	.quad	.LFB144
	.quad	.LFE144-.LFB144
	.quad	.LFB145
	.quad	.LFE145-.LFB145
	.quad	.LFB146
	.quad	.LFE146-.LFB146
	.quad	.LFB147
	.quad	.LFE147-.LFB147
	.quad	.LFB148
	.quad	.LFE148-.LFB148
	.quad	.LFB149
	.quad	.LFE149-.LFB149
	.quad	.LFB150
	.quad	.LFE150-.LFB150
	.quad	.LFB151
	.quad	.LFE151-.LFB151
	.quad	.LFB152
	.quad	.LFE152-.LFB152
	.quad	.LFB153
	.quad	.LFE153-.LFB153
	.quad	.LFB154
	.quad	.LFE154-.LFB154
	.quad	.LFB155
	.quad	.LFE155-.LFB155
	.quad	.LFB156
	.quad	.LFE156-.LFB156
	.quad	.LFB157
	.quad	.LFE157-.LFB157
	.quad	.LFB158
	.quad	.LFE158-.LFB158
	.quad	.LFB159
	.quad	.LFE159-.LFB159
	.quad	.LFB160
	.quad	.LFE160-.LFB160
	.quad	.LFB161
	.quad	.LFE161-.LFB161
	.quad	.LFB162
	.quad	.LFE162-.LFB162
	.quad	.LFB163
	.quad	.LFE163-.LFB163
	.quad	.LFB164
	.quad	.LFE164-.LFB164
	.quad	.LFB165
	.quad	.LFE165-.LFB165
	.quad	.LFB166
	.quad	.LFE166-.LFB166
	.quad	.LFB167
	.quad	.LFE167-.LFB167
	.quad	.LFB168
	.quad	.LFE168-.LFB168
	.quad	.LFB169
	.quad	.LFE169-.LFB169
	.quad	.LFB170
	.quad	.LFE170-.LFB170
	.quad	.LFB171
	.quad	.LFE171-.LFB171
	.quad	.LFB172
	.quad	.LFE172-.LFB172
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LBB2
	.quad	.LBE2
	.quad	.LBB3
	.quad	.LBE3
	.quad	0
	.quad	0
	.quad	.LBB4
	.quad	.LBE4
	.quad	.LBB8
	.quad	.LBE8
	.quad	0
	.quad	0
	.quad	.LBB5
	.quad	.LBE5
	.quad	.LBB6
	.quad	.LBE6
	.quad	0
	.quad	0
	.quad	.LBB12
	.quad	.LBE12
	.quad	.LBB15
	.quad	.LBE15
	.quad	0
	.quad	0
	.quad	.LBB13
	.quad	.LBE13
	.quad	.LBB14
	.quad	.LBE14
	.quad	0
	.quad	0
	.quad	.LBB16
	.quad	.LBE16
	.quad	.LBB17
	.quad	.LBE17
	.quad	0
	.quad	0
	.quad	.LBB18
	.quad	.LBE18
	.quad	.LBB20
	.quad	.LBE20
	.quad	0
	.quad	0
	.quad	.LFB88
	.quad	.LFE88
	.quad	.LFB89
	.quad	.LFE89
	.quad	.LFB90
	.quad	.LFE90
	.quad	.LFB93
	.quad	.LFE93
	.quad	.LFB94
	.quad	.LFE94
	.quad	.LFB95
	.quad	.LFE95
	.quad	.LFB96
	.quad	.LFE96
	.quad	.LFB128
	.quad	.LFE128
	.quad	.LFB129
	.quad	.LFE129
	.quad	.LFB130
	.quad	.LFE130
	.quad	.LFB131
	.quad	.LFE131
	.quad	.LFB132
	.quad	.LFE132
	.quad	.LFB133
	.quad	.LFE133
	.quad	.LFB134
	.quad	.LFE134
	.quad	.LFB135
	.quad	.LFE135
	.quad	.LFB136
	.quad	.LFE136
	.quad	.LFB137
	.quad	.LFE137
	.quad	.LFB138
	.quad	.LFE138
	.quad	.LFB139
	.quad	.LFE139
	.quad	.LFB140
	.quad	.LFE140
	.quad	.LFB141
	.quad	.LFE141
	.quad	.LFB142
	.quad	.LFE142
	.quad	.LFB143
	.quad	.LFE143
	.quad	.LFB144
	.quad	.LFE144
	.quad	.LFB145
	.quad	.LFE145
	.quad	.LFB146
	.quad	.LFE146
	.quad	.LFB147
	.quad	.LFE147
	.quad	.LFB148
	.quad	.LFE148
	.quad	.LFB149
	.quad	.LFE149
	.quad	.LFB150
	.quad	.LFE150
	.quad	.LFB151
	.quad	.LFE151
	.quad	.LFB152
	.quad	.LFE152
	.quad	.LFB153
	.quad	.LFE153
	.quad	.LFB154
	.quad	.LFE154
	.quad	.LFB155
	.quad	.LFE155
	.quad	.LFB156
	.quad	.LFE156
	.quad	.LFB157
	.quad	.LFE157
	.quad	.LFB158
	.quad	.LFE158
	.quad	.LFB159
	.quad	.LFE159
	.quad	.LFB160
	.quad	.LFE160
	.quad	.LFB161
	.quad	.LFE161
	.quad	.LFB162
	.quad	.LFE162
	.quad	.LFB163
	.quad	.LFE163
	.quad	.LFB164
	.quad	.LFE164
	.quad	.LFB165
	.quad	.LFE165
	.quad	.LFB166
	.quad	.LFE166
	.quad	.LFB167
	.quad	.LFE167
	.quad	.LFB168
	.quad	.LFE168
	.quad	.LFB169
	.quad	.LFE169
	.quad	.LFB170
	.quad	.LFE170
	.quad	.LFB171
	.quad	.LFE171
	.quad	.LFB172
	.quad	.LFE172
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF130:
	.string	"aws_lc_0_38_0_CBB_cleanup"
.LASF86:
	.string	"aws_lc_0_38_0_CBB_add_u64"
.LASF8:
	.string	"size_t"
.LASF116:
	.string	"aws_lc_0_38_0_CBB_len"
.LASF32:
	.string	"cbb_child_st"
.LASF20:
	.string	"uint64_t"
.LASF9:
	.string	"__uint8_t"
.LASF35:
	.string	"pending_len_len"
.LASF134:
	.string	"cbb_on_error"
.LASF81:
	.string	"started"
.LASF25:
	.string	"is_child"
.LASF13:
	.string	"__int64_t"
.LASF139:
	.string	"OPENSSL_memcpy"
.LASF80:
	.string	"aws_lc_0_38_0_CBB_add_asn1_uint64_with_tag"
.LASF2:
	.string	"long long int"
.LASF7:
	.string	"signed char"
.LASF46:
	.string	"qsort"
.LASF84:
	.string	"__PRETTY_FUNCTION__"
.LASF30:
	.string	"can_resize"
.LASF145:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/cbb.c"
.LASF93:
	.string	"cbb_add_u"
.LASF29:
	.string	"cbb_buffer_st"
.LASF0:
	.string	"long int"
.LASF62:
	.string	"a_ptr"
.LASF60:
	.string	"buf_len"
.LASF131:
	.string	"aws_lc_0_38_0_CBB_init_fixed"
.LASF51:
	.string	"memcpy"
.LASF45:
	.string	"aws_lc_0_38_0_CBS_data"
.LASF71:
	.string	"parse_dotted_decimal"
.LASF10:
	.string	"short int"
.LASF18:
	.string	"uint16_t"
.LASF59:
	.string	"num_children"
.LASF37:
	.string	"double"
.LASF42:
	.string	"aws_lc_0_38_0_OPENSSL_memdup"
.LASF76:
	.string	"aws_lc_0_38_0_CBB_add_asn1_int64_with_tag"
.LASF85:
	.string	"aws_lc_0_38_0_CBB_add_u64le"
.LASF77:
	.string	"bytes"
.LASF82:
	.string	"byte"
.LASF12:
	.string	"__uint32_t"
.LASF115:
	.string	"prefix_bytes"
.LASF21:
	.string	"long long unsigned int"
.LASF73:
	.string	"value"
.LASF135:
	.string	"cbb_init"
.LASF132:
	.string	"aws_lc_0_38_0_CBB_init"
.LASF109:
	.string	"aws_lc_0_38_0_CBB_add_u16_length_prefixed"
.LASF38:
	.string	"__int128"
.LASF128:
	.string	"newbuf"
.LASF1:
	.string	"long unsigned int"
.LASF91:
	.string	"aws_lc_0_38_0_CBB_add_u16"
.LASF74:
	.string	"aws_lc_0_38_0_CBB_add_asn1_octet_string"
.LASF79:
	.string	"aws_lc_0_38_0_CBB_add_asn1_int64"
.LASF28:
	.string	"data"
.LASF87:
	.string	"aws_lc_0_38_0_CBB_add_u32le"
.LASF5:
	.string	"short unsigned int"
.LASF90:
	.string	"aws_lc_0_38_0_CBB_add_u16le"
.LASF98:
	.string	"out_data"
.LASF133:
	.string	"initial_capacity"
.LASF118:
	.string	"aws_lc_0_38_0_CBB_flush"
.LASF67:
	.string	"aws_lc_0_38_0_CBB_flush_asn1_set_of"
.LASF96:
	.string	"newlen"
.LASF138:
	.string	"OPENSSL_memmove"
.LASF66:
	.string	"min_len"
.LASF53:
	.string	"aws_lc_0_38_0_OPENSSL_realloc"
.LASF33:
	.string	"base"
.LASF107:
	.string	"copy"
.LASF43:
	.string	"aws_lc_0_38_0_CBS_get_any_asn1_element"
.LASF122:
	.string	"cbb_get_base"
.LASF89:
	.string	"aws_lc_0_38_0_CBB_add_u24"
.LASF111:
	.string	"cbb_add_length_prefixed"
.LASF55:
	.string	"aws_lc_0_38_0_OPENSSL_free"
.LASF26:
	.string	"cbb_st"
.LASF127:
	.string	"newcap"
.LASF41:
	.string	"aws_lc_0_38_0_OPENSSL_calloc"
.LASF65:
	.string	"b_len"
.LASF110:
	.string	"aws_lc_0_38_0_CBB_add_u8_length_prefixed"
.LASF57:
	.string	"aws_lc_0_38_0_OPENSSL_malloc"
.LASF14:
	.string	"__uint64_t"
.LASF36:
	.string	"pending_is_asn1"
.LASF54:
	.string	"aws_lc_0_38_0_ERR_put_error"
.LASF104:
	.string	"tag_bits"
.LASF121:
	.string	"extra_bytes"
.LASF56:
	.string	"__assert_fail"
.LASF88:
	.string	"aws_lc_0_38_0_CBB_add_u32"
.LASF39:
	.string	"__int128 unsigned"
.LASF61:
	.string	"children"
.LASF112:
	.string	"cbb_add_child"
.LASF40:
	.string	"_Bool"
.LASF68:
	.string	"aws_lc_0_38_0_CBB_add_asn1_oid_from_text"
.LASF4:
	.string	"unsigned char"
.LASF146:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF24:
	.string	"child"
.LASF49:
	.string	"aws_lc_0_38_0_CBS_get_u8"
.LASF27:
	.string	"cbs_st"
.LASF125:
	.string	"cbb_buffer_add"
.LASF70:
	.string	"compare_set_of_element"
.LASF114:
	.string	"is_asn1"
.LASF23:
	.string	"CBS_ASN1_TAG"
.LASF144:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF31:
	.string	"error"
.LASF75:
	.string	"data_len"
.LASF140:
	.string	"OPENSSL_memcmp"
.LASF106:
	.string	"add_base128_integer"
.LASF48:
	.string	"aws_lc_0_38_0_CBS_len"
.LASF126:
	.string	"cbb_buffer_reserve"
.LASF19:
	.string	"uint32_t"
.LASF99:
	.string	"aws_lc_0_38_0_CBB_add_space"
.LASF3:
	.string	"long double"
.LASF119:
	.string	"child_start"
.LASF95:
	.string	"aws_lc_0_38_0_CBB_did_write"
.LASF15:
	.string	"char"
.LASF137:
	.string	"OPENSSL_memset"
.LASF69:
	.string	"text"
.LASF103:
	.string	"out_contents"
.LASF6:
	.string	"unsigned int"
.LASF11:
	.string	"__uint16_t"
.LASF44:
	.string	"memcmp"
.LASF34:
	.string	"offset"
.LASF120:
	.string	"initial_length_byte"
.LASF63:
	.string	"b_ptr"
.LASF50:
	.string	"aws_lc_0_38_0_CBS_get_u64_decimal"
.LASF83:
	.string	"aws_lc_0_38_0_CBB_add_asn1_uint64"
.LASF97:
	.string	"aws_lc_0_38_0_CBB_reserve"
.LASF117:
	.string	"aws_lc_0_38_0_CBB_data"
.LASF72:
	.string	"aws_lc_0_38_0_CBB_add_asn1_bool"
.LASF108:
	.string	"aws_lc_0_38_0_CBB_add_u24_length_prefixed"
.LASF94:
	.string	"len_len"
.LASF58:
	.string	"memset"
.LASF141:
	.string	"CRYPTO_bswap8"
.LASF100:
	.string	"aws_lc_0_38_0_CBB_add_zeros"
.LASF52:
	.string	"memmove"
.LASF129:
	.string	"aws_lc_0_38_0_CBB_discard_child"
.LASF105:
	.string	"tag_number"
.LASF17:
	.string	"uint8_t"
.LASF47:
	.string	"aws_lc_0_38_0_CBS_init"
.LASF101:
	.string	"aws_lc_0_38_0_CBB_add_bytes"
.LASF22:
	.string	"__compar_fn_t"
.LASF64:
	.string	"a_len"
.LASF143:
	.string	"CRYPTO_bswap2"
.LASF142:
	.string	"CRYPTO_bswap4"
.LASF124:
	.string	"out_len"
.LASF16:
	.string	"int64_t"
.LASF136:
	.string	"aws_lc_0_38_0_CBB_zero"
.LASF78:
	.string	"start"
.LASF123:
	.string	"aws_lc_0_38_0_CBB_finish"
.LASF113:
	.string	"out_child"
.LASF92:
	.string	"aws_lc_0_38_0_CBB_add_u8"
.LASF102:
	.string	"aws_lc_0_38_0_CBB_add_asn1"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
