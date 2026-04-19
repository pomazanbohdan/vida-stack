	.file	"cbs.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/cbs.c"
	.section	.text.CRYPTO_bswap2,"ax",@progbits
	.type	CRYPTO_bswap2, @function
CRYPTO_bswap2:
.LFB223:
	.file 2 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/../asn1/../internal.h"
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
.LFE223:
	.size	CRYPTO_bswap2, .-CRYPTO_bswap2
	.section	.text.CRYPTO_bswap4,"ax",@progbits
	.type	CRYPTO_bswap4, @function
CRYPTO_bswap4:
.LFB224:
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
.LFE224:
	.size	CRYPTO_bswap4, .-CRYPTO_bswap4
	.section	.text.CRYPTO_bswap8,"ax",@progbits
	.type	CRYPTO_bswap8, @function
CRYPTO_bswap8:
.LFB225:
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
.LFE225:
	.size	CRYPTO_bswap8, .-CRYPTO_bswap8
	.section	.text.OPENSSL_memchr,"ax",@progbits
	.type	OPENSSL_memchr, @function
OPENSSL_memchr:
.LFB227:
	.loc 2 935 68
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
	.loc 2 936 6
	cmpq	$0, -24(%rbp)
	jne	.L8
	.loc 2 937 12
	movl	$0, %eax
	jmp	.L9
.L8:
	.loc 2 943 10
	movq	-24(%rbp), %rdx
	movl	-12(%rbp), %ecx
	movq	-8(%rbp), %rax
	movl	%ecx, %esi
	movq	%rax, %rdi
	call	memchr@PLT
.L9:
	.loc 2 944 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE227:
	.size	OPENSSL_memchr, .-OPENSSL_memchr
	.section	.text.OPENSSL_memcpy,"ax",@progbits
	.type	OPENSSL_memcpy, @function
OPENSSL_memcpy:
.LFB229:
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
.LFE229:
	.size	OPENSSL_memcpy, .-OPENSSL_memcpy
	.section	.text.aws_lc_0_38_0_CBS_init,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_init
	.type	aws_lc_0_38_0_CBS_init, @function
aws_lc_0_38_0_CBS_init:
.LFB263:
	.loc 1 34 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 1 35 13
	movq	-8(%rbp), %rax
	movq	-16(%rbp), %rdx
	movq	%rdx, (%rax)
	.loc 1 36 12
	movq	-8(%rbp), %rax
	movq	-24(%rbp), %rdx
	movq	%rdx, 8(%rax)
	.loc 1 37 1
	nop
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE263:
	.size	aws_lc_0_38_0_CBS_init, .-aws_lc_0_38_0_CBS_init
	.section	.text.cbs_get,"ax",@progbits
	.type	cbs_get, @function
cbs_get:
.LFB264:
	.loc 1 39 59
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 1 40 10
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 40 6
	cmpq	-24(%rbp), %rax
	jnb	.L15
	.loc 1 41 12
	movl	$0, %eax
	jmp	.L16
.L15:
	.loc 1 44 11
	movq	-8(%rbp), %rax
	movq	(%rax), %rdx
	.loc 1 44 6
	movq	-16(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 45 6
	movq	-8(%rbp), %rax
	movq	(%rax), %rdx
	.loc 1 45 13
	movq	-24(%rbp), %rax
	addq	%rax, %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 46 6
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 46 12
	subq	-24(%rbp), %rax
	movq	%rax, %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, 8(%rax)
	.loc 1 47 10
	movl	$1, %eax
.L16:
	.loc 1 48 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE264:
	.size	cbs_get, .-cbs_get
	.section	.text.aws_lc_0_38_0_CBS_skip,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_skip
	.type	aws_lc_0_38_0_CBS_skip, @function
aws_lc_0_38_0_CBS_skip:
.LFB265:
	.loc 1 50 36
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 52 10
	movq	-32(%rbp), %rdx
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get
	.loc 1 53 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE265:
	.size	aws_lc_0_38_0_CBS_skip, .-aws_lc_0_38_0_CBS_skip
	.section	.text.aws_lc_0_38_0_CBS_data,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_data
	.type	aws_lc_0_38_0_CBS_data, @function
aws_lc_0_38_0_CBS_data:
.LFB266:
	.loc 1 55 41
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	.loc 1 55 53
	movq	-8(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 55 61
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE266:
	.size	aws_lc_0_38_0_CBS_data, .-aws_lc_0_38_0_CBS_data
	.section	.text.aws_lc_0_38_0_CBS_len,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_len
	.type	aws_lc_0_38_0_CBS_len, @function
aws_lc_0_38_0_CBS_len:
.LFB267:
	.loc 1 57 32
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	.loc 1 57 44
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 57 51
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE267:
	.size	aws_lc_0_38_0_CBS_len, .-aws_lc_0_38_0_CBS_len
	.section	.text.aws_lc_0_38_0_CBS_stow,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_stow
	.type	aws_lc_0_38_0_CBS_stow, @function
aws_lc_0_38_0_CBS_stow:
.LFB268:
	.loc 1 59 66
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
	.loc 1 60 3
	movq	-16(%rbp), %rax
	movq	(%rax), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_free@PLT
	.loc 1 61 12
	movq	-16(%rbp), %rax
	movq	$0, (%rax)
	.loc 1 62 12
	movq	-24(%rbp), %rax
	movq	$0, (%rax)
	.loc 1 64 10
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 64 6
	testq	%rax, %rax
	jne	.L24
	.loc 1 65 12
	movl	$1, %eax
	jmp	.L25
.L24:
	.loc 1 67 43
	movq	-8(%rbp), %rax
	movq	8(%rax), %rdx
	.loc 1 67 32
	movq	-8(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 67 14
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_memdup@PLT
	.loc 1 67 12 discriminator 1
	movq	-16(%rbp), %rdx
	movq	%rax, (%rdx)
	.loc 1 68 7
	movq	-16(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 68 6
	testq	%rax, %rax
	jne	.L26
	.loc 1 69 12
	movl	$0, %eax
	jmp	.L25
.L26:
	.loc 1 71 17
	movq	-8(%rbp), %rax
	movq	8(%rax), %rdx
	.loc 1 71 12
	movq	-24(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 72 10
	movl	$1, %eax
.L25:
	.loc 1 73 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE268:
	.size	aws_lc_0_38_0_CBS_stow, .-aws_lc_0_38_0_CBS_stow
	.section	.text.aws_lc_0_38_0_CBS_strdup,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_strdup
	.type	aws_lc_0_38_0_CBS_strdup, @function
aws_lc_0_38_0_CBS_strdup:
.LFB269:
	.loc 1 75 48
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 76 7
	movq	-16(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 76 6
	testq	%rax, %rax
	je	.L28
	.loc 1 77 5
	movq	-16(%rbp), %rax
	movq	(%rax), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_free@PLT
.L28:
	.loc 1 79 58
	movq	-8(%rbp), %rax
	movq	8(%rax), %rdx
	.loc 1 79 47
	movq	-8(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 79 14
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_strndup@PLT
	.loc 1 79 12 discriminator 1
	movq	-16(%rbp), %rdx
	movq	%rax, (%rdx)
	.loc 1 80 11
	movq	-16(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 80 20
	testq	%rax, %rax
	setne	%al
	movzbl	%al, %eax
	.loc 1 81 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE269:
	.size	aws_lc_0_38_0_CBS_strdup, .-aws_lc_0_38_0_CBS_strdup
	.section	.text.aws_lc_0_38_0_CBS_contains_zero_byte,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_contains_zero_byte
	.type	aws_lc_0_38_0_CBS_contains_zero_byte, @function
aws_lc_0_38_0_CBS_contains_zero_byte:
.LFB270:
	.loc 1 83 44
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	.loc 1 84 42
	movq	-8(%rbp), %rax
	movq	8(%rax), %rdx
	.loc 1 84 28
	movq	-8(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 84 10
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memchr
	.loc 1 84 49 discriminator 1
	testq	%rax, %rax
	setne	%al
	movzbl	%al, %eax
	.loc 1 85 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE270:
	.size	aws_lc_0_38_0_CBS_contains_zero_byte, .-aws_lc_0_38_0_CBS_contains_zero_byte
	.section	.text.aws_lc_0_38_0_CBS_mem_equal,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_mem_equal
	.type	aws_lc_0_38_0_CBS_mem_equal, @function
aws_lc_0_38_0_CBS_mem_equal:
.LFB271:
	.loc 1 87 68
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
	.loc 1 88 17
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 88 6
	cmpq	%rax, -24(%rbp)
	je	.L33
	.loc 1 89 12
	movl	$0, %eax
	jmp	.L34
.L33:
	.loc 1 91 27
	movq	-8(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 91 10
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rcx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_memcmp@PLT
	.loc 1 91 46 discriminator 1
	testl	%eax, %eax
	sete	%al
	movzbl	%al, %eax
.L34:
	.loc 1 92 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE271:
	.size	aws_lc_0_38_0_CBS_mem_equal, .-aws_lc_0_38_0_CBS_mem_equal
	.section	.text.cbs_get_u,"ax",@progbits
	.type	cbs_get_u, @function
cbs_get_u:
.LFB272:
	.loc 1 94 59
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$56, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	movq	%rdx, -56(%rbp)
	.loc 1 95 12
	movq	$0, -8(%rbp)
	.loc 1 98 8
	movq	-56(%rbp), %rdx
	leaq	-24(%rbp), %rcx
	movq	-40(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get
	.loc 1 98 6 discriminator 1
	testl	%eax, %eax
	jne	.L36
	.loc 1 99 12
	movl	$0, %eax
	jmp	.L40
.L36:
.LBB2:
	.loc 1 101 15
	movq	$0, -16(%rbp)
	.loc 1 101 3
	jmp	.L38
.L39:
	.loc 1 102 12
	salq	$8, -8(%rbp)
	.loc 1 103 19
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	movzbl	%al, %eax
	.loc 1 103 12
	orq	%rax, -8(%rbp)
	.loc 1 101 32 discriminator 3
	addq	$1, -16(%rbp)
.L38:
	.loc 1 101 24 discriminator 1
	movq	-16(%rbp), %rax
	cmpq	-56(%rbp), %rax
	jb	.L39
.LBE2:
	.loc 1 105 8
	movq	-48(%rbp), %rax
	movq	-8(%rbp), %rdx
	movq	%rdx, (%rax)
	.loc 1 106 10
	movl	$1, %eax
.L40:
	.loc 1 107 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE272:
	.size	cbs_get_u, .-cbs_get_u
	.section	.text.aws_lc_0_38_0_CBS_get_u8,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u8
	.type	aws_lc_0_38_0_CBS_get_u8, @function
aws_lc_0_38_0_CBS_get_u8:
.LFB273:
	.loc 1 109 40
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 111 8
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movl	$1, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get
	.loc 1 111 6 discriminator 1
	testl	%eax, %eax
	jne	.L42
	.loc 1 112 12
	movl	$0, %eax
	jmp	.L44
.L42:
	.loc 1 114 10
	movq	-8(%rbp), %rax
	movzbl	(%rax), %edx
	.loc 1 114 8
	movq	-32(%rbp), %rax
	movb	%dl, (%rax)
	.loc 1 115 10
	movl	$1, %eax
.L44:
	.loc 1 116 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE273:
	.size	aws_lc_0_38_0_CBS_get_u8, .-aws_lc_0_38_0_CBS_get_u8
	.section	.text.aws_lc_0_38_0_CBS_get_u16,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u16
	.type	aws_lc_0_38_0_CBS_get_u16, @function
aws_lc_0_38_0_CBS_get_u16:
.LFB274:
	.loc 1 118 42
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 120 8
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movl	$2, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get_u
	.loc 1 120 6 discriminator 1
	testl	%eax, %eax
	jne	.L46
	.loc 1 121 12
	movl	$0, %eax
	jmp	.L48
.L46:
	.loc 1 123 8
	movq	-8(%rbp), %rax
	movl	%eax, %edx
	movq	-32(%rbp), %rax
	movw	%dx, (%rax)
	.loc 1 124 10
	movl	$1, %eax
.L48:
	.loc 1 125 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE274:
	.size	aws_lc_0_38_0_CBS_get_u16, .-aws_lc_0_38_0_CBS_get_u16
	.section	.text.aws_lc_0_38_0_CBS_get_u16le,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u16le
	.type	aws_lc_0_38_0_CBS_get_u16le, @function
aws_lc_0_38_0_CBS_get_u16le:
.LFB275:
	.loc 1 127 44
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 128 8
	movq	-16(%rbp), %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u16@PLT
	.loc 1 128 6 discriminator 1
	testl	%eax, %eax
	jne	.L50
	.loc 1 129 12
	movl	$0, %eax
	jmp	.L51
.L50:
	.loc 1 131 24
	movq	-16(%rbp), %rax
	movzwl	(%rax), %eax
	.loc 1 131 10
	movzwl	%ax, %eax
	movl	%eax, %edi
	call	CRYPTO_bswap2
	.loc 1 131 8 discriminator 1
	movq	-16(%rbp), %rdx
	movw	%ax, (%rdx)
	.loc 1 132 10
	movl	$1, %eax
.L51:
	.loc 1 133 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE275:
	.size	aws_lc_0_38_0_CBS_get_u16le, .-aws_lc_0_38_0_CBS_get_u16le
	.section	.text.aws_lc_0_38_0_CBS_get_u24,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u24
	.type	aws_lc_0_38_0_CBS_get_u24, @function
aws_lc_0_38_0_CBS_get_u24:
.LFB276:
	.loc 1 135 42
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 137 8
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movl	$3, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get_u
	.loc 1 137 6 discriminator 1
	testl	%eax, %eax
	jne	.L53
	.loc 1 138 12
	movl	$0, %eax
	jmp	.L55
.L53:
	.loc 1 140 10
	movq	-8(%rbp), %rax
	movl	%eax, %edx
	.loc 1 140 8
	movq	-32(%rbp), %rax
	movl	%edx, (%rax)
	.loc 1 141 10
	movl	$1, %eax
.L55:
	.loc 1 142 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE276:
	.size	aws_lc_0_38_0_CBS_get_u24, .-aws_lc_0_38_0_CBS_get_u24
	.section	.text.aws_lc_0_38_0_CBS_get_u32,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u32
	.type	aws_lc_0_38_0_CBS_get_u32, @function
aws_lc_0_38_0_CBS_get_u32:
.LFB277:
	.loc 1 144 42
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 146 8
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movl	$4, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get_u
	.loc 1 146 6 discriminator 1
	testl	%eax, %eax
	jne	.L57
	.loc 1 147 12
	movl	$0, %eax
	jmp	.L59
.L57:
	.loc 1 149 10
	movq	-8(%rbp), %rax
	movl	%eax, %edx
	.loc 1 149 8
	movq	-32(%rbp), %rax
	movl	%edx, (%rax)
	.loc 1 150 10
	movl	$1, %eax
.L59:
	.loc 1 151 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE277:
	.size	aws_lc_0_38_0_CBS_get_u32, .-aws_lc_0_38_0_CBS_get_u32
	.section	.text.aws_lc_0_38_0_CBS_get_u32le,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u32le
	.type	aws_lc_0_38_0_CBS_get_u32le, @function
aws_lc_0_38_0_CBS_get_u32le:
.LFB278:
	.loc 1 153 44
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 154 8
	movq	-16(%rbp), %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u32@PLT
	.loc 1 154 6 discriminator 1
	testl	%eax, %eax
	jne	.L61
	.loc 1 155 12
	movl	$0, %eax
	jmp	.L62
.L61:
	.loc 1 157 10
	movq	-16(%rbp), %rax
	movl	(%rax), %eax
	movl	%eax, %edi
	call	CRYPTO_bswap4
	.loc 1 157 8 discriminator 1
	movq	-16(%rbp), %rdx
	movl	%eax, (%rdx)
	.loc 1 158 10
	movl	$1, %eax
.L62:
	.loc 1 159 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE278:
	.size	aws_lc_0_38_0_CBS_get_u32le, .-aws_lc_0_38_0_CBS_get_u32le
	.section	.text.aws_lc_0_38_0_CBS_get_u64,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u64
	.type	aws_lc_0_38_0_CBS_get_u64, @function
aws_lc_0_38_0_CBS_get_u64:
.LFB279:
	.loc 1 161 42
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 161 51
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$8, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get_u
	.loc 1 161 75
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE279:
	.size	aws_lc_0_38_0_CBS_get_u64, .-aws_lc_0_38_0_CBS_get_u64
	.section	.text.aws_lc_0_38_0_CBS_get_u64le,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u64le
	.type	aws_lc_0_38_0_CBS_get_u64le, @function
aws_lc_0_38_0_CBS_get_u64le:
.LFB280:
	.loc 1 163 44
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 164 8
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$8, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get_u
	.loc 1 164 6 discriminator 1
	testl	%eax, %eax
	jne	.L66
	.loc 1 165 12
	movl	$0, %eax
	jmp	.L67
.L66:
	.loc 1 167 10
	movq	-16(%rbp), %rax
	movq	(%rax), %rax
	movq	%rax, %rdi
	call	CRYPTO_bswap8
	.loc 1 167 8 discriminator 1
	movq	-16(%rbp), %rdx
	movq	%rax, (%rdx)
	.loc 1 168 10
	movl	$1, %eax
.L67:
	.loc 1 169 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE280:
	.size	aws_lc_0_38_0_CBS_get_u64le, .-aws_lc_0_38_0_CBS_get_u64le
	.section	.text.aws_lc_0_38_0_CBS_get_last_u8,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_last_u8
	.type	aws_lc_0_38_0_CBS_get_last_u8, @function
aws_lc_0_38_0_CBS_get_last_u8:
.LFB281:
	.loc 1 171 45
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 172 10
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 172 6
	testq	%rax, %rax
	jne	.L69
	.loc 1 173 12
	movl	$0, %eax
	jmp	.L70
.L69:
	.loc 1 175 13
	movq	-8(%rbp), %rax
	movq	(%rax), %rdx
	.loc 1 175 23
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 175 19
	subq	$1, %rax
	addq	%rdx, %rax
	movzbl	(%rax), %edx
	.loc 1 175 8
	movq	-16(%rbp), %rax
	movb	%dl, (%rax)
	.loc 1 176 6
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 176 11
	leaq	-1(%rax), %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, 8(%rax)
	.loc 1 177 10
	movl	$1, %eax
.L70:
	.loc 1 178 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE281:
	.size	aws_lc_0_38_0_CBS_get_last_u8, .-aws_lc_0_38_0_CBS_get_last_u8
	.section	.text.aws_lc_0_38_0_CBS_get_bytes,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_bytes
	.type	aws_lc_0_38_0_CBS_get_bytes, @function
aws_lc_0_38_0_CBS_get_bytes:
.LFB282:
	.loc 1 180 51
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
	.loc 1 182 8
	movq	-40(%rbp), %rdx
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get
	.loc 1 182 6 discriminator 1
	testl	%eax, %eax
	jne	.L72
	.loc 1 183 12
	movl	$0, %eax
	jmp	.L74
.L72:
	.loc 1 185 3
	movq	-8(%rbp), %rcx
	movq	-40(%rbp), %rdx
	movq	-32(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_init@PLT
	.loc 1 186 10
	movl	$1, %eax
.L74:
	.loc 1 187 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE282:
	.size	aws_lc_0_38_0_CBS_get_bytes, .-aws_lc_0_38_0_CBS_get_bytes
	.section	.text.aws_lc_0_38_0_CBS_copy_bytes,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_copy_bytes
	.type	aws_lc_0_38_0_CBS_copy_bytes, @function
aws_lc_0_38_0_CBS_copy_bytes:
.LFB283:
	.loc 1 189 56
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
	.loc 1 191 8
	movq	-40(%rbp), %rdx
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get
	.loc 1 191 6 discriminator 1
	testl	%eax, %eax
	jne	.L76
	.loc 1 192 12
	movl	$0, %eax
	jmp	.L78
.L76:
	.loc 1 194 3
	movq	-8(%rbp), %rcx
	movq	-40(%rbp), %rdx
	movq	-32(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 195 10
	movl	$1, %eax
.L78:
	.loc 1 196 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE283:
	.size	aws_lc_0_38_0_CBS_copy_bytes, .-aws_lc_0_38_0_CBS_copy_bytes
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/cbs.c"
.LC1:
	.string	"len_len <= 3"
	.section	.text.cbs_get_length_prefixed,"ax",@progbits
	.type	cbs_get_length_prefixed, @function
cbs_get_length_prefixed:
.LFB284:
	.loc 1 198 72
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
	.loc 1 200 8
	movq	-40(%rbp), %rdx
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get_u
	.loc 1 200 6 discriminator 1
	testl	%eax, %eax
	jne	.L80
	.loc 1 201 12
	movl	$0, %eax
	jmp	.L83
.L80:
	.loc 1 205 3
	cmpq	$3, -40(%rbp)
	jbe	.L82
	.loc 1 205 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.4(%rip), %rax
	movq	%rax, %rcx
	movl	$205, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC1(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L82:
	.loc 1 206 10 is_stmt 1
	movq	-8(%rbp), %rdx
	movq	-32(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_bytes@PLT
.L83:
	.loc 1 207 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE284:
	.size	cbs_get_length_prefixed, .-cbs_get_length_prefixed
	.section	.text.aws_lc_0_38_0_CBS_get_u8_length_prefixed,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u8_length_prefixed
	.type	aws_lc_0_38_0_CBS_get_u8_length_prefixed, @function
aws_lc_0_38_0_CBS_get_u8_length_prefixed:
.LFB285:
	.loc 1 209 52
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 210 10
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$1, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get_length_prefixed
	.loc 1 211 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE285:
	.size	aws_lc_0_38_0_CBS_get_u8_length_prefixed, .-aws_lc_0_38_0_CBS_get_u8_length_prefixed
	.section	.text.aws_lc_0_38_0_CBS_get_u16_length_prefixed,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u16_length_prefixed
	.type	aws_lc_0_38_0_CBS_get_u16_length_prefixed, @function
aws_lc_0_38_0_CBS_get_u16_length_prefixed:
.LFB286:
	.loc 1 213 53
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 214 10
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$2, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get_length_prefixed
	.loc 1 215 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE286:
	.size	aws_lc_0_38_0_CBS_get_u16_length_prefixed, .-aws_lc_0_38_0_CBS_get_u16_length_prefixed
	.section	.text.aws_lc_0_38_0_CBS_get_u24_length_prefixed,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u24_length_prefixed
	.type	aws_lc_0_38_0_CBS_get_u24_length_prefixed, @function
aws_lc_0_38_0_CBS_get_u24_length_prefixed:
.LFB287:
	.loc 1 217 53
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 218 10
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$3, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get_length_prefixed
	.loc 1 219 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE287:
	.size	aws_lc_0_38_0_CBS_get_u24_length_prefixed, .-aws_lc_0_38_0_CBS_get_u24_length_prefixed
	.section	.text.aws_lc_0_38_0_CBS_get_until_first,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_until_first
	.type	aws_lc_0_38_0_CBS_get_until_first, @function
aws_lc_0_38_0_CBS_get_until_first:
.LFB288:
	.loc 1 221 56
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%r12
	pushq	%rbx
	subq	$48, %rsp
	.cfi_offset 12, -24
	.cfi_offset 3, -32
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	movl	%edx, %eax
	movb	%al, -52(%rbp)
	.loc 1 222 26
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, %r12
	.loc 1 222 26 is_stmt 0 discriminator 1
	movzbl	-52(%rbp), %ebx
	.loc 1 222 41 is_stmt 1 discriminator 1
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	.loc 1 222 26 discriminator 2
	movq	%r12, %rdx
	movl	%ebx, %esi
	movq	%rax, %rdi
	call	OPENSSL_memchr
	movq	%rax, -24(%rbp)
	.loc 1 223 6
	cmpq	$0, -24(%rbp)
	jne	.L91
	.loc 1 224 12
	movl	$0, %eax
	jmp	.L92
.L91:
	.loc 1 226 42
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	.loc 1 226 40 discriminator 1
	movq	-24(%rbp), %rdx
	subq	%rax, %rdx
	.loc 1 226 10 discriminator 1
	movq	-48(%rbp), %rcx
	movq	-40(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_bytes@PLT
.L92:
	.loc 1 227 1
	addq	$48, %rsp
	popq	%rbx
	popq	%r12
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE288:
	.size	aws_lc_0_38_0_CBS_get_until_first, .-aws_lc_0_38_0_CBS_get_until_first
	.section	.text.aws_lc_0_38_0_CBS_get_u64_decimal,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_u64_decimal
	.type	aws_lc_0_38_0_CBS_get_u64_decimal, @function
aws_lc_0_38_0_CBS_get_u64_decimal:
.LFB289:
	.loc 1 229 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 230 12
	movq	$0, -8(%rbp)
	.loc 1 231 7
	movl	$0, -12(%rbp)
	.loc 1 232 9
	jmp	.L94
.L101:
.LBB3:
	.loc 1 233 30
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	.loc 1 233 13 discriminator 1
	movzbl	(%rax), %eax
	movb	%al, -13(%rbp)
	.loc 1 234 10
	movzbl	-13(%rbp), %eax
	movl	%eax, %edi
	call	aws_lc_0_38_0_OPENSSL_isdigit@PLT
	.loc 1 234 8 discriminator 1
	testl	%eax, %eax
	je	.L102
	.loc 1 237 5
	movq	-24(%rbp), %rax
	movl	$1, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_skip@PLT
	.loc 1 238 8
	cmpq	$0, -8(%rbp)
	jne	.L97
	.loc 1 239 17
	cmpl	$0, -12(%rbp)
	jne	.L98
.L97:
	.loc 1 239 32 discriminator 1
	movabsq	$1844674407370955161, %rax
	cmpq	-8(%rbp), %rax
	jb	.L98
	.loc 1 242 11
	movq	-8(%rbp), %rdx
	movq	%rdx, %rax
	salq	$2, %rax
	addq	%rdx, %rax
	addq	%rax, %rax
	movq	%rax, %rdx
	.loc 1 242 34
	movzbl	-13(%rbp), %eax
	subl	$48, %eax
	cltq
	.loc 1 242 29
	notq	%rax
	.loc 1 241 29
	cmpq	%rdx, %rax
	jnb	.L99
.L98:
	.loc 1 243 14
	movl	$0, %eax
	jmp	.L100
.L99:
	.loc 1 245 11
	movq	-8(%rbp), %rdx
	movq	%rdx, %rax
	salq	$2, %rax
	addq	%rdx, %rax
	addq	%rax, %rax
	movq	%rax, %rdx
	.loc 1 245 21
	movzbl	-13(%rbp), %eax
	subl	$48, %eax
	cltq
	.loc 1 245 7
	addq	%rdx, %rax
	movq	%rax, -8(%rbp)
	.loc 1 246 16
	movl	$1, -12(%rbp)
.L94:
.LBE3:
	.loc 1 232 10
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 232 23 discriminator 1
	testq	%rax, %rax
	jne	.L101
	jmp	.L96
.L102:
.LBB4:
	.loc 1 235 7
	nop
.L96:
.LBE4:
	.loc 1 249 8
	movq	-32(%rbp), %rax
	movq	-8(%rbp), %rdx
	movq	%rdx, (%rax)
	.loc 1 250 10
	movl	-12(%rbp), %eax
.L100:
	.loc 1 251 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE289:
	.size	aws_lc_0_38_0_CBS_get_u64_decimal, .-aws_lc_0_38_0_CBS_get_u64_decimal
	.section	.text.parse_base128_integer,"ax",@progbits
	.type	parse_base128_integer, @function
parse_base128_integer:
.LFB290:
	.loc 1 256 59
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 257 12
	movq	$0, -8(%rbp)
.L108:
	.loc 1 260 10
	leaq	-9(%rbp), %rdx
	movq	-24(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 260 8 discriminator 1
	testl	%eax, %eax
	jne	.L104
	.loc 1 261 14
	movl	$0, %eax
	jmp	.L109
.L104:
	.loc 1 263 12
	movq	-8(%rbp), %rax
	shrq	$57, %rax
	.loc 1 263 8
	testq	%rax, %rax
	je	.L106
	.loc 1 265 14
	movl	$0, %eax
	jmp	.L109
.L106:
	.loc 1 267 8
	cmpq	$0, -8(%rbp)
	jne	.L107
	.loc 1 267 21 discriminator 1
	movzbl	-9(%rbp), %eax
	.loc 1 267 16 discriminator 1
	cmpb	$-128, %al
	jne	.L107
	.loc 1 269 14
	movl	$0, %eax
	jmp	.L109
.L107:
	.loc 1 271 12
	movq	-8(%rbp), %rax
	salq	$7, %rax
	movq	%rax, %rdx
	.loc 1 271 23
	movzbl	-9(%rbp), %eax
	movzbl	%al, %eax
	andl	$127, %eax
	.loc 1 271 7
	orq	%rdx, %rax
	movq	%rax, -8(%rbp)
	.loc 1 274 12
	movzbl	-9(%rbp), %eax
	testb	%al, %al
	js	.L108
	.loc 1 276 8
	movq	-32(%rbp), %rax
	movq	-8(%rbp), %rdx
	movq	%rdx, (%rax)
	.loc 1 277 10
	movl	$1, %eax
.L109:
	.loc 1 278 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE290:
	.size	parse_base128_integer, .-parse_base128_integer
	.section	.text.parse_asn1_tag,"ax",@progbits
	.type	parse_asn1_tag, @function
parse_asn1_tag:
.LFB291:
	.loc 1 280 78
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	movl	%edx, -52(%rbp)
	.loc 1 282 8
	leaq	-9(%rbp), %rdx
	movq	-40(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 282 6 discriminator 1
	testl	%eax, %eax
	jne	.L111
	.loc 1 283 12
	movl	$0, %eax
	jmp	.L117
.L111:
	.loc 1 292 54
	movzbl	-9(%rbp), %eax
	movzbl	%al, %eax
	sall	$24, %eax
	.loc 1 292 16
	andl	$-536870912, %eax
	movl	%eax, -8(%rbp)
	.loc 1 293 38
	movzbl	-9(%rbp), %eax
	movzbl	%al, %eax
	.loc 1 293 16
	andl	$31, %eax
	movl	%eax, -4(%rbp)
	.loc 1 294 6
	cmpl	$31, -4(%rbp)
	jne	.L113
.LBB5:
	.loc 1 296 10
	leaq	-24(%rbp), %rdx
	movq	-40(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	parse_base128_integer
	.loc 1 296 8 discriminator 1
	testl	%eax, %eax
	je	.L114
	.loc 1 298 11
	movq	-24(%rbp), %rax
	.loc 1 296 41 discriminator 1
	cmpq	$536870911, %rax
	ja	.L114
	.loc 1 300 11
	movq	-24(%rbp), %rax
	.loc 1 298 38
	cmpq	$30, %rax
	ja	.L115
.L114:
	.loc 1 301 14
	movl	$0, %eax
	jmp	.L117
.L115:
	.loc 1 303 18
	movq	-24(%rbp), %rax
	.loc 1 303 16
	movl	%eax, -4(%rbp)
.L113:
.LBE5:
	.loc 1 306 7
	movl	-4(%rbp), %eax
	orl	%eax, -8(%rbp)
	.loc 1 311 6
	cmpl	$0, -52(%rbp)
	jne	.L116
	.loc 1 311 33 discriminator 1
	movl	-8(%rbp), %eax
	andl	$-536870913, %eax
	.loc 1 311 25 discriminator 1
	testl	%eax, %eax
	jne	.L116
	.loc 1 312 12
	movl	$0, %eax
	jmp	.L117
.L116:
	.loc 1 315 8
	movq	-48(%rbp), %rax
	movl	-8(%rbp), %edx
	movl	%edx, (%rax)
	.loc 1 316 10
	movl	$1, %eax
.L117:
	.loc 1 317 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE291:
	.size	parse_asn1_tag, .-parse_asn1_tag
	.section	.rodata
.LC2:
	.string	"out_ber_found == NULL"
.LC3:
	.string	"out_indefinite == NULL"
	.section	.text.aws_lc_0_38_0_cbs_get_any_asn1_element,"ax",@progbits
	.globl	aws_lc_0_38_0_cbs_get_any_asn1_element
	.type	aws_lc_0_38_0_cbs_get_any_asn1_element, @function
aws_lc_0_38_0_cbs_get_any_asn1_element:
.LFB292:
	.loc 1 321 92
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%rbx
	subq	$136, %rsp
	.cfi_offset 3, -24
	movq	%rdi, -104(%rbp)
	movq	%rsi, -112(%rbp)
	movq	%rdx, -120(%rbp)
	movq	%rcx, -128(%rbp)
	movq	%r8, -136(%rbp)
	movq	%r9, -144(%rbp)
	.loc 1 322 7
	movq	-104(%rbp), %rax
	movq	8(%rax), %rdx
	movq	(%rax), %rax
	movq	%rax, -64(%rbp)
	movq	%rdx, -56(%rbp)
	.loc 1 325 6
	cmpq	$0, -112(%rbp)
	jne	.L119
	.loc 1 326 9
	leaq	-80(%rbp), %rax
	movq	%rax, -112(%rbp)
.L119:
	.loc 1 328 6
	cmpl	$0, 16(%rbp)
	je	.L120
	.loc 1 329 20
	movq	-136(%rbp), %rax
	movl	$0, (%rax)
	.loc 1 330 21
	movq	-144(%rbp), %rax
	movl	$0, (%rax)
	jmp	.L121
.L120:
	.loc 1 332 5
	cmpq	$0, -136(%rbp)
	je	.L122
	.loc 1 332 5 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.3(%rip), %rax
	movq	%rax, %rcx
	movl	$332, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC2(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L122:
	.loc 1 333 5 is_stmt 1
	cmpq	$0, -144(%rbp)
	je	.L121
	.loc 1 333 5 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.3(%rip), %rax
	movq	%rax, %rcx
	movl	$333, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC3(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L121:
	.loc 1 337 8 is_stmt 1
	movl	24(%rbp), %edx
	leaq	-84(%rbp), %rcx
	leaq	-64(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	parse_asn1_tag
	.loc 1 337 6 discriminator 1
	testl	%eax, %eax
	jne	.L123
	.loc 1 338 12
	movl	$0, %eax
	jmp	.L141
.L123:
	.loc 1 340 6
	cmpq	$0, -120(%rbp)
	je	.L125
	.loc 1 341 14
	movl	-84(%rbp), %edx
	movq	-120(%rbp), %rax
	movl	%edx, (%rax)
.L125:
	.loc 1 345 8
	leaq	-85(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 345 6 discriminator 1
	testl	%eax, %eax
	jne	.L126
	.loc 1 346 12
	movl	$0, %eax
	jmp	.L141
.L126:
	.loc 1 349 23
	movq	-104(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, %rbx
	.loc 1 349 38 discriminator 1
	leaq	-64(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 349 10 discriminator 2
	subq	%rax, %rbx
	movq	%rbx, %rdx
	movq	%rdx, -32(%rbp)
	.loc 1 354 28
	movzbl	-85(%rbp), %eax
	.loc 1 354 6
	testb	%al, %al
	js	.L127
	.loc 1 356 12
	movzbl	-85(%rbp), %eax
	movzbl	%al, %edx
	.loc 1 356 9
	movq	-32(%rbp), %rax
	addq	%rdx, %rax
	movq	%rax, -24(%rbp)
	.loc 1 357 8
	cmpq	$0, -128(%rbp)
	je	.L128
	.loc 1 358 23
	movq	-128(%rbp), %rax
	movq	-32(%rbp), %rdx
	movq	%rdx, (%rax)
	jmp	.L128
.L127:
.LBB6:
	.loc 1 364 42
	movzbl	-85(%rbp), %eax
	movzbl	%al, %eax
	.loc 1 364 18
	andl	$127, %eax
	movq	%rax, -40(%rbp)
	.loc 1 367 8
	cmpl	$0, 16(%rbp)
	je	.L129
	.loc 1 367 24 discriminator 1
	movl	-84(%rbp), %eax
	andl	$536870912, %eax
	.loc 1 367 16 discriminator 1
	testl	%eax, %eax
	je	.L129
	.loc 1 367 53 discriminator 2
	cmpq	$0, -40(%rbp)
	jne	.L129
	.loc 1 369 10
	cmpq	$0, -128(%rbp)
	je	.L130
	.loc 1 370 25
	movq	-128(%rbp), %rax
	movq	-32(%rbp), %rdx
	movq	%rdx, (%rax)
.L130:
	.loc 1 372 22
	movq	-136(%rbp), %rax
	movl	$1, (%rax)
	.loc 1 373 23
	movq	-144(%rbp), %rax
	movl	$1, (%rax)
	.loc 1 374 14
	movq	-32(%rbp), %rdx
	movq	-112(%rbp), %rcx
	movq	-104(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_bytes@PLT
	jmp	.L141
.L129:
	.loc 1 380 8
	cmpq	$0, -40(%rbp)
	je	.L132
	.loc 1 380 24 discriminator 1
	cmpq	$4, -40(%rbp)
	jbe	.L133
.L132:
	.loc 1 381 14
	movl	$0, %eax
	jmp	.L141
.L133:
	.loc 1 383 10
	movq	-40(%rbp), %rdx
	leaq	-96(%rbp), %rcx
	leaq	-64(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_get_u
	.loc 1 383 8 discriminator 1
	testl	%eax, %eax
	jne	.L134
	.loc 1 384 14
	movl	$0, %eax
	jmp	.L141
.L134:
	.loc 1 390 15
	movq	-96(%rbp), %rax
	.loc 1 390 8
	cmpq	$127, %rax
	ja	.L135
	.loc 1 392 10
	cmpl	$0, 16(%rbp)
	je	.L136
	.loc 1 393 24
	movq	-136(%rbp), %rax
	movl	$1, (%rax)
	jmp	.L135
.L136:
	.loc 1 395 16
	movl	$0, %eax
	jmp	.L141
.L135:
	.loc 1 398 16
	movq	-96(%rbp), %rdx
	.loc 1 398 31
	movq	-40(%rbp), %rax
	subq	$1, %rax
	.loc 1 398 16
	sall	$3, %eax
	shrx	%rax, %rdx, %rax
	.loc 1 398 8
	testq	%rax, %rax
	jne	.L137
	.loc 1 400 10
	cmpl	$0, 16(%rbp)
	je	.L138
	.loc 1 401 24
	movq	-136(%rbp), %rax
	movl	$1, (%rax)
	jmp	.L137
.L138:
	.loc 1 403 16
	movl	$0, %eax
	jmp	.L141
.L137:
	.loc 1 406 9
	movq	-96(%rbp), %rax
	movq	%rax, -24(%rbp)
	.loc 1 407 13
	movq	-24(%rbp), %rdx
	movq	-32(%rbp), %rax
	addq	%rax, %rdx
	.loc 1 407 26
	movq	-40(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 407 8
	cmpq	-24(%rbp), %rax
	jnb	.L139
	.loc 1 409 14
	movl	$0, %eax
	jmp	.L141
.L139:
	.loc 1 411 23
	movq	-32(%rbp), %rdx
	movq	-40(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 411 9
	addq	%rax, -24(%rbp)
	.loc 1 412 8
	cmpq	$0, -128(%rbp)
	je	.L128
	.loc 1 413 36
	movq	-32(%rbp), %rdx
	movq	-40(%rbp), %rax
	addq	%rax, %rdx
	.loc 1 413 23
	movq	-128(%rbp), %rax
	movq	%rdx, (%rax)
.L128:
.LBE6:
	.loc 1 417 10
	movq	-24(%rbp), %rdx
	movq	-112(%rbp), %rcx
	movq	-104(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_bytes@PLT
.L141:
	.loc 1 418 1
	movq	-8(%rbp), %rbx
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE292:
	.size	aws_lc_0_38_0_cbs_get_any_asn1_element, .-aws_lc_0_38_0_cbs_get_any_asn1_element
	.section	.rodata
.LC4:
	.string	"0"
	.section	.text.aws_lc_0_38_0_CBS_get_any_asn1,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_any_asn1
	.type	aws_lc_0_38_0_CBS_get_any_asn1, @function
aws_lc_0_38_0_CBS_get_any_asn1:
.LFB293:
	.loc 1 420 65
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
	.loc 1 422 8
	leaq	-8(%rbp), %rcx
	movq	-40(%rbp), %rdx
	movq	-32(%rbp), %rsi
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_any_asn1_element@PLT
	.loc 1 422 6 discriminator 1
	testl	%eax, %eax
	jne	.L143
	.loc 1 423 12
	movl	$0, %eax
	jmp	.L146
.L143:
	.loc 1 426 8
	movq	-8(%rbp), %rdx
	movq	-32(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_skip@PLT
	.loc 1 426 6 discriminator 1
	testl	%eax, %eax
	jne	.L145
	.loc 1 427 5
	leaq	__PRETTY_FUNCTION__.2(%rip), %rax
	movq	%rax, %rcx
	movl	$427, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC4(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L145:
	.loc 1 431 10
	movl	$1, %eax
.L146:
	.loc 1 432 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE293:
	.size	aws_lc_0_38_0_CBS_get_any_asn1, .-aws_lc_0_38_0_CBS_get_any_asn1
	.section	.text.aws_lc_0_38_0_CBS_get_any_asn1_element,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_any_asn1_element
	.type	aws_lc_0_38_0_CBS_get_any_asn1_element, @function
aws_lc_0_38_0_CBS_get_any_asn1_element:
.LFB294:
	.loc 1 435 54
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
	movq	%rcx, -32(%rbp)
	.loc 1 436 10
	movq	-32(%rbp), %rcx
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rsi
	movq	-8(%rbp), %rax
	pushq	$0
	pushq	$0
	movl	$0, %r9d
	movl	$0, %r8d
	movq	%rax, %rdi
	call	aws_lc_0_38_0_cbs_get_any_asn1_element@PLT
	addq	$16, %rsp
	.loc 1 438 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE294:
	.size	aws_lc_0_38_0_CBS_get_any_asn1_element, .-aws_lc_0_38_0_CBS_get_any_asn1_element
	.section	.text.aws_lc_0_38_0_CBS_get_any_ber_asn1_element,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_any_ber_asn1_element
	.type	aws_lc_0_38_0_CBS_get_any_ber_asn1_element, @function
aws_lc_0_38_0_CBS_get_any_ber_asn1_element:
.LFB295:
	.loc 1 442 55
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	movq	%rdx, -40(%rbp)
	movq	%rcx, -48(%rbp)
	movq	%r8, -56(%rbp)
	movq	%r9, -64(%rbp)
	.loc 1 444 10
	cmpq	$0, -56(%rbp)
	jne	.L150
	.loc 1 444 10 is_stmt 0 discriminator 1
	leaq	-4(%rbp), %rax
	jmp	.L151
.L150:
	.loc 1 444 10 discriminator 2
	movq	-56(%rbp), %rax
.L151:
	.loc 1 444 10 discriminator 4
	movq	-64(%rbp), %r8
	movq	-48(%rbp), %rcx
	movq	-40(%rbp), %rdx
	movq	-32(%rbp), %rsi
	movq	-24(%rbp), %rdi
	pushq	$0
	pushq	$1
	movq	%r8, %r9
	movq	%rax, %r8
	call	aws_lc_0_38_0_cbs_get_any_asn1_element@PLT
	addq	$16, %rsp
	.loc 1 448 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE295:
	.size	aws_lc_0_38_0_CBS_get_any_ber_asn1_element, .-aws_lc_0_38_0_CBS_get_any_ber_asn1_element
	.section	.text.cbs_get_asn1,"ax",@progbits
	.type	cbs_get_asn1, @function
cbs_get_asn1:
.LFB296:
	.loc 1 451 42
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	movl	%edx, -52(%rbp)
	movl	%ecx, -56(%rbp)
	.loc 1 456 6
	cmpq	$0, -48(%rbp)
	jne	.L154
	.loc 1 457 9
	leaq	-32(%rbp), %rax
	movq	%rax, -48(%rbp)
.L154:
	.loc 1 460 8
	leaq	-8(%rbp), %rcx
	leaq	-12(%rbp), %rdx
	movq	-48(%rbp), %rsi
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_any_asn1_element@PLT
	.loc 1 460 6 discriminator 1
	testl	%eax, %eax
	je	.L155
	.loc 1 461 11
	movl	-12(%rbp), %eax
	.loc 1 460 62 discriminator 1
	cmpl	%eax, -52(%rbp)
	je	.L156
.L155:
	.loc 1 462 12
	movl	$0, %eax
	jmp	.L159
.L156:
	.loc 1 465 6
	cmpl	$0, -56(%rbp)
	je	.L158
	.loc 1 465 23 discriminator 1
	movq	-8(%rbp), %rdx
	movq	-48(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_skip@PLT
	.loc 1 465 19 discriminator 1
	testl	%eax, %eax
	jne	.L158
	.loc 1 466 5
	leaq	__PRETTY_FUNCTION__.1(%rip), %rax
	movq	%rax, %rcx
	movl	$466, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC4(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L158:
	.loc 1 470 10
	movl	$1, %eax
.L159:
	.loc 1 471 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE296:
	.size	cbs_get_asn1, .-cbs_get_asn1
	.section	.text.aws_lc_0_38_0_CBS_get_asn1,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_asn1
	.type	aws_lc_0_38_0_CBS_get_asn1, @function
aws_lc_0_38_0_CBS_get_asn1:
.LFB297:
	.loc 1 473 62
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movl	%edx, -20(%rbp)
	.loc 1 474 10
	movl	-20(%rbp), %edx
	movq	-16(%rbp), %rsi
	movq	-8(%rbp), %rax
	movl	$1, %ecx
	movq	%rax, %rdi
	call	cbs_get_asn1
	.loc 1 475 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE297:
	.size	aws_lc_0_38_0_CBS_get_asn1, .-aws_lc_0_38_0_CBS_get_asn1
	.section	.text.aws_lc_0_38_0_CBS_get_asn1_element,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_asn1_element
	.type	aws_lc_0_38_0_CBS_get_asn1_element, @function
aws_lc_0_38_0_CBS_get_asn1_element:
.LFB298:
	.loc 1 477 70
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movl	%edx, -20(%rbp)
	.loc 1 478 10
	movl	-20(%rbp), %edx
	movq	-16(%rbp), %rsi
	movq	-8(%rbp), %rax
	movl	$0, %ecx
	movq	%rax, %rdi
	call	cbs_get_asn1
	.loc 1 479 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE298:
	.size	aws_lc_0_38_0_CBS_get_asn1_element, .-aws_lc_0_38_0_CBS_get_asn1_element
	.section	.text.aws_lc_0_38_0_CBS_peek_asn1_tag,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_peek_asn1_tag
	.type	aws_lc_0_38_0_CBS_peek_asn1_tag, @function
aws_lc_0_38_0_CBS_peek_asn1_tag:
.LFB299:
	.loc 1 481 63
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -40(%rbp)
	movl	%esi, -44(%rbp)
	.loc 1 482 7
	movq	-40(%rbp), %rax
	movq	8(%rax), %rdx
	movq	(%rax), %rax
	movq	%rax, -16(%rbp)
	movq	%rdx, -8(%rbp)
	.loc 1 484 10
	leaq	-20(%rbp), %rcx
	leaq	-16(%rbp), %rax
	movl	$0, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	parse_asn1_tag
	.loc 1 484 69 discriminator 1
	testl	%eax, %eax
	je	.L165
	.loc 1 484 82 discriminator 1
	movl	-20(%rbp), %eax
	.loc 1 484 69 discriminator 1
	cmpl	%eax, -44(%rbp)
	jne	.L165
	.loc 1 484 69 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 1 484 69
	jmp	.L167
.L165:
	.loc 1 484 69 discriminator 4
	movl	$0, %eax
.L167:
	.loc 1 485 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE299:
	.size	aws_lc_0_38_0_CBS_peek_asn1_tag, .-aws_lc_0_38_0_CBS_peek_asn1_tag
	.section	.text.aws_lc_0_38_0_CBS_get_asn1_uint64,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_asn1_uint64
	.type	aws_lc_0_38_0_CBS_get_asn1_uint64, @function
aws_lc_0_38_0_CBS_get_asn1_uint64:
.LFB300:
	.loc 1 487 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -56(%rbp)
	movq	%rsi, -64(%rbp)
	.loc 1 489 8
	leaq	-48(%rbp), %rcx
	movq	-56(%rbp), %rax
	movl	$2, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1@PLT
	.loc 1 489 6 discriminator 1
	testl	%eax, %eax
	je	.L169
	.loc 1 490 8
	leaq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_is_unsigned_asn1_integer@PLT
	.loc 1 489 52 discriminator 1
	testl	%eax, %eax
	jne	.L170
.L169:
	.loc 1 491 12
	movl	$0, %eax
	jmp	.L175
.L170:
	.loc 1 494 8
	movq	-64(%rbp), %rax
	movq	$0, (%rax)
	.loc 1 495 25
	leaq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	movq	%rax, -16(%rbp)
	.loc 1 496 16
	leaq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, -24(%rbp)
.LBB7:
	.loc 1 497 15
	movq	$0, -8(%rbp)
	.loc 1 497 3
	jmp	.L172
.L174:
	.loc 1 498 10
	movq	-64(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 498 15
	shrq	$56, %rax
	.loc 1 498 8
	testq	%rax, %rax
	je	.L173
	.loc 1 500 14
	movl	$0, %eax
	jmp	.L175
.L173:
	.loc 1 502 5
	movq	-64(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 502 10
	salq	$8, %rax
	movq	%rax, %rdx
	movq	-64(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 503 5
	movq	-64(%rbp), %rax
	movq	(%rax), %rdx
	.loc 1 503 17
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	addq	%rcx, %rax
	movzbl	(%rax), %eax
	movzbl	%al, %eax
	.loc 1 503 10
	orq	%rax, %rdx
	movq	-64(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 497 32 discriminator 2
	addq	$1, -8(%rbp)
.L172:
	.loc 1 497 24 discriminator 1
	movq	-8(%rbp), %rax
	cmpq	-24(%rbp), %rax
	jb	.L174
.LBE7:
	.loc 1 506 10
	movl	$1, %eax
.L175:
	.loc 1 507 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE300:
	.size	aws_lc_0_38_0_CBS_get_asn1_uint64, .-aws_lc_0_38_0_CBS_get_asn1_uint64
	.section	.text.aws_lc_0_38_0_CBS_get_asn1_int64,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_asn1_int64
	.type	aws_lc_0_38_0_CBS_get_asn1_int64, @function
aws_lc_0_38_0_CBS_get_asn1_int64:
.LFB301:
	.loc 1 509 48
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$80, %rsp
	movq	%rdi, -72(%rbp)
	movq	%rsi, -80(%rbp)
	.loc 1 512 8
	leaq	-48(%rbp), %rcx
	movq	-72(%rbp), %rax
	movl	$2, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1@PLT
	.loc 1 512 6 discriminator 1
	testl	%eax, %eax
	je	.L177
	.loc 1 513 8
	leaq	-28(%rbp), %rdx
	leaq	-48(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_is_valid_asn1_integer@PLT
	.loc 1 512 52 discriminator 1
	testl	%eax, %eax
	jne	.L178
.L177:
	.loc 1 514 12
	movl	$0, %eax
	jmp	.L186
.L178:
	.loc 1 516 25
	leaq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	movq	%rax, -16(%rbp)
	.loc 1 517 22
	leaq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, -24(%rbp)
	.loc 1 518 6
	cmpq	$8, -24(%rbp)
	jbe	.L180
	.loc 1 519 12
	movl	$0, %eax
	jmp	.L186
.L180:
	.loc 1 522 35
	movl	-28(%rbp), %eax
	.loc 1 522 3
	testl	%eax, %eax
	je	.L181
	.loc 1 522 3 is_stmt 0 discriminator 1
	movl	$255, %ecx
	jmp	.L182
.L181:
	.loc 1 522 3 discriminator 2
	movl	$0, %ecx
.L182:
	.loc 1 522 3 discriminator 4
	leaq	-56(%rbp), %rax
	movl	$8, %edx
	movl	%ecx, %esi
	movq	%rax, %rdi
	call	memset@PLT
.LBB8:
	.loc 1 525 15 is_stmt 1
	movq	$0, -8(%rbp)
	.loc 1 525 3
	jmp	.L183
.L185:
	.loc 1 533 31
	movq	-24(%rbp), %rax
	subq	-8(%rbp), %rax
	.loc 1 533 26
	leaq	-1(%rax), %rdx
	movq	-16(%rbp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	.loc 1 533 20
	leaq	-56(%rbp), %rcx
	movq	-8(%rbp), %rdx
	addq	%rcx, %rdx
	movb	%al, (%rdx)
	.loc 1 525 55 discriminator 4
	addq	$1, -8(%rbp)
.L183:
	.loc 1 525 30 discriminator 1
	movq	-8(%rbp), %rax
	cmpq	-24(%rbp), %rax
	jnb	.L184
	.loc 1 525 30 is_stmt 0 discriminator 3
	cmpq	$7, -8(%rbp)
	jbe	.L185
.L184:
.LBE8:
	.loc 1 537 3 is_stmt 1
	movq	-56(%rbp), %rdx
	movq	-80(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 538 10
	movl	$1, %eax
.L186:
	.loc 1 539 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE301:
	.size	aws_lc_0_38_0_CBS_get_asn1_int64, .-aws_lc_0_38_0_CBS_get_asn1_int64
	.section	.text.aws_lc_0_38_0_CBS_get_asn1_bool,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_asn1_bool
	.type	aws_lc_0_38_0_CBS_get_asn1_bool, @function
aws_lc_0_38_0_CBS_get_asn1_bool:
.LFB302:
	.loc 1 541 43
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	.loc 1 543 8
	leaq	-32(%rbp), %rcx
	movq	-40(%rbp), %rax
	movl	$1, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1@PLT
	.loc 1 543 6 discriminator 1
	testl	%eax, %eax
	je	.L188
	.loc 1 543 55 discriminator 1
	leaq	-32(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 543 52 discriminator 1
	cmpq	$1, %rax
	je	.L189
.L188:
	.loc 1 544 12
	movl	$0, %eax
	jmp	.L192
.L189:
	.loc 1 547 26
	leaq	-32(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	.loc 1 547 17 discriminator 1
	movzbl	(%rax), %eax
	movb	%al, -1(%rbp)
	.loc 1 548 6
	cmpb	$0, -1(%rbp)
	je	.L191
	.loc 1 548 18 discriminator 1
	cmpb	$-1, -1(%rbp)
	je	.L191
	.loc 1 549 12
	movl	$0, %eax
	jmp	.L192
.L191:
	.loc 1 552 10
	cmpb	$0, -1(%rbp)
	setne	%al
	movzbl	%al, %edx
	.loc 1 552 8
	movq	-48(%rbp), %rax
	movl	%edx, (%rax)
	.loc 1 553 10
	movl	$1, %eax
.L192:
	.loc 1 554 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE302:
	.size	aws_lc_0_38_0_CBS_get_asn1_bool, .-aws_lc_0_38_0_CBS_get_asn1_bool
	.section	.text.aws_lc_0_38_0_CBS_get_optional_asn1,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_optional_asn1
	.type	aws_lc_0_38_0_CBS_get_optional_asn1, @function
aws_lc_0_38_0_CBS_get_optional_asn1:
.LFB303:
	.loc 1 557 45
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
	movl	%ecx, -44(%rbp)
	.loc 1 558 7
	movl	$0, -4(%rbp)
	.loc 1 560 7
	movl	-44(%rbp), %edx
	movq	-24(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_peek_asn1_tag@PLT
	.loc 1 560 6 discriminator 1
	testl	%eax, %eax
	je	.L194
	.loc 1 561 10
	movl	-44(%rbp), %edx
	movq	-32(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1@PLT
	.loc 1 561 8 discriminator 1
	testl	%eax, %eax
	jne	.L195
	.loc 1 562 14
	movl	$0, %eax
	jmp	.L196
.L195:
	.loc 1 564 13
	movl	$1, -4(%rbp)
.L194:
	.loc 1 567 6
	cmpq	$0, -40(%rbp)
	je	.L197
	.loc 1 568 18
	movq	-40(%rbp), %rax
	movl	-4(%rbp), %edx
	movl	%edx, (%rax)
.L197:
	.loc 1 571 10
	movl	$1, %eax
.L196:
	.loc 1 572 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE303:
	.size	aws_lc_0_38_0_CBS_get_optional_asn1, .-aws_lc_0_38_0_CBS_get_optional_asn1
	.section	.rodata
.LC5:
	.string	"out"
	.section	.text.aws_lc_0_38_0_CBS_get_optional_asn1_octet_string,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_optional_asn1_octet_string
	.type	aws_lc_0_38_0_CBS_get_optional_asn1_octet_string, @function
aws_lc_0_38_0_CBS_get_optional_asn1_octet_string:
.LFB304:
	.loc 1 575 58
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
	movl	%ecx, -60(%rbp)
	.loc 1 578 8
	movl	-60(%rbp), %ecx
	leaq	-20(%rbp), %rdx
	leaq	-16(%rbp), %rsi
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_optional_asn1@PLT
	.loc 1 578 6 discriminator 1
	testl	%eax, %eax
	jne	.L199
	.loc 1 579 12
	movl	$0, %eax
	jmp	.L206
.L199:
	.loc 1 581 7
	movl	-20(%rbp), %eax
	.loc 1 581 6
	testl	%eax, %eax
	je	.L201
	.loc 1 582 5
	cmpq	$0, -48(%rbp)
	jne	.L202
	.loc 1 582 5 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$582, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC5(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L202:
	.loc 1 583 10 is_stmt 1
	movq	-48(%rbp), %rcx
	leaq	-16(%rbp), %rax
	movl	$4, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1@PLT
	.loc 1 583 8 discriminator 1
	testl	%eax, %eax
	je	.L203
	.loc 1 584 9
	leaq	-16(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 583 58 discriminator 1
	testq	%rax, %rax
	je	.L204
.L203:
	.loc 1 585 14
	movl	$0, %eax
	jmp	.L206
.L201:
	.loc 1 588 5
	movq	-48(%rbp), %rax
	movl	$0, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_init@PLT
.L204:
	.loc 1 590 6
	cmpq	$0, -56(%rbp)
	je	.L205
	.loc 1 591 18
	movl	-20(%rbp), %edx
	movq	-56(%rbp), %rax
	movl	%edx, (%rax)
.L205:
	.loc 1 593 10
	movl	$1, %eax
.L206:
	.loc 1 594 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE304:
	.size	aws_lc_0_38_0_CBS_get_optional_asn1_octet_string, .-aws_lc_0_38_0_CBS_get_optional_asn1_octet_string
	.section	.text.aws_lc_0_38_0_CBS_get_optional_asn1_uint64,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_optional_asn1_uint64
	.type	aws_lc_0_38_0_CBS_get_optional_asn1_uint64, @function
aws_lc_0_38_0_CBS_get_optional_asn1_uint64:
.LFB305:
	.loc 1 597 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	movl	%edx, -52(%rbp)
	movq	%rcx, -64(%rbp)
	.loc 1 600 8
	movl	-52(%rbp), %ecx
	leaq	-20(%rbp), %rdx
	leaq	-16(%rbp), %rsi
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_optional_asn1@PLT
	.loc 1 600 6 discriminator 1
	testl	%eax, %eax
	jne	.L208
	.loc 1 601 12
	movl	$0, %eax
	jmp	.L213
.L208:
	.loc 1 603 7
	movl	-20(%rbp), %eax
	.loc 1 603 6
	testl	%eax, %eax
	je	.L210
	.loc 1 604 10
	movq	-48(%rbp), %rdx
	leaq	-16(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1_uint64@PLT
	.loc 1 604 8 discriminator 1
	testl	%eax, %eax
	je	.L211
	.loc 1 604 46 discriminator 1
	leaq	-16(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 604 43 discriminator 1
	testq	%rax, %rax
	je	.L212
.L211:
	.loc 1 605 14
	movl	$0, %eax
	jmp	.L213
.L210:
	.loc 1 608 10
	movq	-48(%rbp), %rax
	movq	-64(%rbp), %rdx
	movq	%rdx, (%rax)
.L212:
	.loc 1 610 10
	movl	$1, %eax
.L213:
	.loc 1 611 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE305:
	.size	aws_lc_0_38_0_CBS_get_optional_asn1_uint64, .-aws_lc_0_38_0_CBS_get_optional_asn1_uint64
	.section	.text.aws_lc_0_38_0_CBS_get_optional_asn1_bool,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_optional_asn1_bool
	.type	aws_lc_0_38_0_CBS_get_optional_asn1_bool, @function
aws_lc_0_38_0_CBS_get_optional_asn1_bool:
.LFB306:
	.loc 1 614 51
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
	movl	%ecx, -88(%rbp)
	.loc 1 617 8
	movl	-84(%rbp), %ecx
	leaq	-52(%rbp), %rdx
	leaq	-32(%rbp), %rsi
	movq	-72(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_optional_asn1@PLT
	.loc 1 617 6 discriminator 1
	testl	%eax, %eax
	jne	.L215
	.loc 1 618 12
	movl	$0, %eax
	jmp	.L223
.L215:
	.loc 1 620 7
	movl	-52(%rbp), %eax
	.loc 1 620 6
	testl	%eax, %eax
	je	.L217
.LBB9:
	.loc 1 623 10
	leaq	-48(%rbp), %rcx
	leaq	-32(%rbp), %rax
	movl	$1, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1@PLT
	.loc 1 623 8 discriminator 1
	testl	%eax, %eax
	je	.L218
	.loc 1 624 9
	leaq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 623 58 discriminator 1
	cmpq	$1, %rax
	jne	.L218
	.loc 1 624 34
	leaq	-32(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 624 31 discriminator 1
	testq	%rax, %rax
	je	.L219
.L218:
	.loc 1 625 14
	movl	$0, %eax
	jmp	.L223
.L219:
	.loc 1 628 32
	leaq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	.loc 1 628 13 discriminator 1
	movzbl	(%rax), %eax
	movb	%al, -1(%rbp)
	.loc 1 629 8
	cmpb	$0, -1(%rbp)
	jne	.L220
	.loc 1 630 12
	movq	-80(%rbp), %rax
	movl	$0, (%rax)
	jmp	.L221
.L220:
	.loc 1 631 15
	cmpb	$-1, -1(%rbp)
	jne	.L222
	.loc 1 632 12
	movq	-80(%rbp), %rax
	movl	$1, (%rax)
	jmp	.L221
.L222:
	.loc 1 634 14
	movl	$0, %eax
	jmp	.L223
.L217:
.LBE9:
	.loc 1 637 10
	movq	-80(%rbp), %rax
	movl	-88(%rbp), %edx
	movl	%edx, (%rax)
.L221:
	.loc 1 639 10
	movl	$1, %eax
.L223:
	.loc 1 640 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE306:
	.size	aws_lc_0_38_0_CBS_get_optional_asn1_bool, .-aws_lc_0_38_0_CBS_get_optional_asn1_bool
	.section	.text.aws_lc_0_38_0_CBS_is_valid_asn1_bitstring,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_is_valid_asn1_bitstring
	.type	aws_lc_0_38_0_CBS_is_valid_asn1_bitstring, @function
aws_lc_0_38_0_CBS_is_valid_asn1_bitstring:
.LFB307:
	.loc 1 642 49
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -40(%rbp)
	.loc 1 643 7
	movq	-40(%rbp), %rax
	movq	8(%rax), %rdx
	movq	(%rax), %rax
	movq	%rax, -16(%rbp)
	movq	%rdx, -8(%rbp)
	.loc 1 645 8
	leaq	-17(%rbp), %rdx
	leaq	-16(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 645 6 discriminator 1
	testl	%eax, %eax
	je	.L225
	.loc 1 645 61 discriminator 1
	movzbl	-17(%rbp), %eax
	.loc 1 645 42 discriminator 1
	cmpb	$7, %al
	jbe	.L226
.L225:
	.loc 1 646 12
	movl	$0, %eax
	jmp	.L231
.L226:
	.loc 1 649 23
	movzbl	-17(%rbp), %eax
	.loc 1 649 6
	testb	%al, %al
	jne	.L228
	.loc 1 650 12
	movl	$1, %eax
	jmp	.L231
.L228:
	.loc 1 655 8
	leaq	-18(%rbp), %rdx
	leaq	-16(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_last_u8@PLT
	.loc 1 655 6 discriminator 1
	testl	%eax, %eax
	je	.L229
	.loc 1 656 13
	movzbl	-18(%rbp), %eax
	movzbl	%al, %edx
	.loc 1 656 19
	movzbl	-17(%rbp), %eax
	movzbl	%al, %eax
	movl	$1, %ecx
	shlx	%eax, %ecx, %eax
	.loc 1 656 39
	subl	$1, %eax
	.loc 1 656 13
	andl	%edx, %eax
	.loc 1 655 36 discriminator 1
	testl	%eax, %eax
	je	.L230
.L229:
	.loc 1 657 12
	movl	$0, %eax
	jmp	.L231
.L230:
	.loc 1 660 10
	movl	$1, %eax
.L231:
	.loc 1 661 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE307:
	.size	aws_lc_0_38_0_CBS_is_valid_asn1_bitstring, .-aws_lc_0_38_0_CBS_is_valid_asn1_bitstring
	.section	.text.aws_lc_0_38_0_CBS_asn1_bitstring_has_bit,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_asn1_bitstring_has_bit
	.type	aws_lc_0_38_0_CBS_asn1_bitstring_has_bit, @function
aws_lc_0_38_0_CBS_asn1_bitstring_has_bit:
.LFB308:
	.loc 1 663 62
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%rbx
	subq	$40, %rsp
	.cfi_offset 3, -24
	movq	%rdi, -40(%rbp)
	movl	%esi, -44(%rbp)
	.loc 1 664 8
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_is_valid_asn1_bitstring@PLT
	.loc 1 664 6 discriminator 1
	testl	%eax, %eax
	jne	.L233
	.loc 1 665 12
	movl	$0, %eax
	jmp	.L234
.L233:
	.loc 1 668 34
	movl	-44(%rbp), %eax
	shrl	$3, %eax
	.loc 1 668 18
	addl	$1, %eax
	movl	%eax, -20(%rbp)
	.loc 1 669 30
	movl	-44(%rbp), %eax
	notl	%eax
	.loc 1 669 18
	andl	$7, %eax
	movl	%eax, -24(%rbp)
	.loc 1 674 19
	movl	-20(%rbp), %ebx
	.loc 1 674 21
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 674 34 discriminator 1
	cmpq	%rax, %rbx
	jnb	.L235
	.loc 1 675 11
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	.loc 1 675 24 discriminator 1
	movl	-20(%rbp), %edx
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	movzbl	%al, %edx
	.loc 1 675 53 discriminator 1
	movl	-24(%rbp), %eax
	sarx	%eax, %edx, %eax
	andl	$1, %eax
	.loc 1 674 34 discriminator 1
	testl	%eax, %eax
	je	.L235
	.loc 1 674 34 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 1 674 34
	jmp	.L234
.L235:
	.loc 1 674 34 discriminator 4
	movl	$0, %eax
.L234:
	.loc 1 676 1 is_stmt 1
	movq	-8(%rbp), %rbx
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE308:
	.size	aws_lc_0_38_0_CBS_asn1_bitstring_has_bit, .-aws_lc_0_38_0_CBS_asn1_bitstring_has_bit
	.section	.text.aws_lc_0_38_0_CBS_is_valid_asn1_integer,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_is_valid_asn1_integer
	.type	aws_lc_0_38_0_CBS_is_valid_asn1_integer, @function
aws_lc_0_38_0_CBS_is_valid_asn1_integer:
.LFB309:
	.loc 1 678 69
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	.loc 1 679 7
	movq	-40(%rbp), %rax
	movq	8(%rax), %rdx
	movq	(%rax), %rax
	movq	%rax, -16(%rbp)
	movq	%rdx, -8(%rbp)
	.loc 1 681 8
	leaq	-17(%rbp), %rdx
	leaq	-16(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 681 6 discriminator 1
	testl	%eax, %eax
	jne	.L238
	.loc 1 682 12
	movl	$0, %eax
	jmp	.L245
.L238:
	.loc 1 684 6
	cmpq	$0, -48(%rbp)
	je	.L240
	.loc 1 685 44
	movzbl	-17(%rbp), %eax
	shrb	$7, %al
	movzbl	%al, %edx
	.loc 1 685 22
	movq	-48(%rbp), %rax
	movl	%edx, (%rax)
.L240:
	.loc 1 687 8
	leaq	-18(%rbp), %rdx
	leaq	-16(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 687 6 discriminator 1
	testl	%eax, %eax
	jne	.L241
	.loc 1 688 12
	movl	$1, %eax
	jmp	.L245
.L241:
	.loc 1 690 19
	movzbl	-17(%rbp), %eax
	.loc 1 690 6
	testb	%al, %al
	jne	.L242
	.loc 1 690 51 discriminator 1
	movzbl	-18(%rbp), %eax
	.loc 1 690 27 discriminator 1
	testb	%al, %al
	jns	.L243
.L242:
	.loc 1 691 19
	movzbl	-17(%rbp), %eax
	.loc 1 690 57 discriminator 3
	cmpb	$-1, %al
	jne	.L244
	.loc 1 691 51
	movzbl	-18(%rbp), %eax
	.loc 1 691 27
	testb	%al, %al
	jns	.L244
.L243:
	.loc 1 692 12
	movl	$0, %eax
	jmp	.L245
.L244:
	.loc 1 694 10
	movl	$1, %eax
.L245:
	.loc 1 695 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE309:
	.size	aws_lc_0_38_0_CBS_is_valid_asn1_integer, .-aws_lc_0_38_0_CBS_is_valid_asn1_integer
	.section	.text.aws_lc_0_38_0_CBS_is_unsigned_asn1_integer,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_is_unsigned_asn1_integer
	.type	aws_lc_0_38_0_CBS_is_unsigned_asn1_integer, @function
aws_lc_0_38_0_CBS_is_unsigned_asn1_integer:
.LFB310:
	.loc 1 697 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	.loc 1 699 10
	leaq	-4(%rbp), %rdx
	movq	-24(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_is_valid_asn1_integer@PLT
	.loc 1 699 55 discriminator 1
	testl	%eax, %eax
	je	.L247
	.loc 1 699 58 discriminator 1
	movl	-4(%rbp), %eax
	.loc 1 699 55 discriminator 1
	testl	%eax, %eax
	jne	.L247
	.loc 1 699 55 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 1 699 55
	jmp	.L249
.L247:
	.loc 1 699 55 discriminator 4
	movl	$0, %eax
.L249:
	.loc 1 700 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE310:
	.size	aws_lc_0_38_0_CBS_is_unsigned_asn1_integer, .-aws_lc_0_38_0_CBS_is_unsigned_asn1_integer
	.section	.rodata
.LC6:
	.string	"%lu"
	.section	.text.add_decimal,"ax",@progbits
	.type	add_decimal, @function
add_decimal:
.LFB311:
	.loc 1 702 46
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	.loc 1 704 3
	movq	-48(%rbp), %rdx
	leaq	-32(%rbp), %rax
	movq	%rdx, %rcx
	leaq	.LC6(%rip), %rdx
	movl	$24, %esi
	movq	%rax, %rdi
	movl	$0, %eax
	call	snprintf@PLT
	.loc 1 705 10
	leaq	-32(%rbp), %rax
	movq	%rax, %rdi
	call	strlen@PLT
	movq	%rax, %rdx
	.loc 1 705 10 is_stmt 0 discriminator 1
	leaq	-32(%rbp), %rcx
	movq	-40(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_bytes@PLT
	.loc 1 706 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE311:
	.size	add_decimal, .-add_decimal
	.section	.text.aws_lc_0_38_0_CBS_is_valid_asn1_oid,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_is_valid_asn1_oid
	.type	aws_lc_0_38_0_CBS_is_valid_asn1_oid, @function
aws_lc_0_38_0_CBS_is_valid_asn1_oid:
.LFB312:
	.loc 1 708 43
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -56(%rbp)
	.loc 1 709 7
	movq	-56(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 709 6 discriminator 1
	testq	%rax, %rax
	jne	.L253
	.loc 1 710 12
	movl	$0, %eax
	jmp	.L258
.L253:
	.loc 1 713 7
	movq	-56(%rbp), %rax
	movq	8(%rax), %rdx
	movq	(%rax), %rax
	movq	%rax, -32(%rbp)
	movq	%rdx, -24(%rbp)
	.loc 1 714 14
	movb	$0, -1(%rbp)
	.loc 1 715 9
	jmp	.L255
.L257:
	.loc 1 721 23
	movzbl	-1(%rbp), %eax
	.loc 1 721 8
	testb	%al, %al
	js	.L256
	.loc 1 721 33 discriminator 1
	movzbl	-33(%rbp), %eax
	.loc 1 721 28 discriminator 1
	cmpb	$-128, %al
	jne	.L256
	.loc 1 722 14
	movl	$0, %eax
	jmp	.L258
.L256:
	.loc 1 724 10
	movzbl	-33(%rbp), %eax
	movb	%al, -1(%rbp)
.L255:
	.loc 1 715 10
	leaq	-33(%rbp), %rdx
	leaq	-32(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 715 10 is_stmt 0 discriminator 1
	testl	%eax, %eax
	jne	.L257
	.loc 1 728 24 is_stmt 1
	movzbl	-1(%rbp), %eax
	notl	%eax
	shrb	$7, %al
	movzbl	%al, %eax
.L258:
	.loc 1 729 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE312:
	.size	aws_lc_0_38_0_CBS_is_valid_asn1_oid, .-aws_lc_0_38_0_CBS_is_valid_asn1_oid
	.section	.rodata
.LC7:
	.string	"2."
	.section	.text.aws_lc_0_38_0_CBS_asn1_oid_to_text,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_asn1_oid_to_text
	.type	aws_lc_0_38_0_CBS_asn1_oid_to_text, @function
aws_lc_0_38_0_CBS_asn1_oid_to_text:
.LFB313:
	.loc 1 731 44
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$112, %rsp
	movq	%rdi, -104(%rbp)
	.loc 1 733 8
	leaq	-48(%rbp), %rax
	movl	$32, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_init@PLT
	.loc 1 733 6 discriminator 1
	testl	%eax, %eax
	je	.L274
	.loc 1 737 7
	movq	-104(%rbp), %rax
	movq	8(%rax), %rdx
	movq	(%rax), %rax
	movq	%rax, -64(%rbp)
	movq	%rdx, -56(%rbp)
	.loc 1 740 8
	leaq	-72(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	parse_base128_integer
	.loc 1 740 6 discriminator 1
	testl	%eax, %eax
	je	.L275
	.loc 1 744 9
	movq	-72(%rbp), %rax
	.loc 1 744 6
	cmpq	$79, %rax
	jbe	.L263
	.loc 1 745 10
	leaq	-48(%rbp), %rax
	movl	$2, %edx
	leaq	.LC7(%rip), %rcx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_bytes@PLT
	.loc 1 745 8 discriminator 1
	testl	%eax, %eax
	je	.L276
	.loc 1 746 10
	movq	-72(%rbp), %rax
	leaq	-80(%rax), %rdx
	leaq	-48(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	add_decimal
	.loc 1 745 56 discriminator 1
	testl	%eax, %eax
	jne	.L267
	.loc 1 747 7
	jmp	.L276
.L263:
	.loc 1 749 15
	movq	-72(%rbp), %rax
	movabsq	$-3689348814741910323, %rdx
	mulq	%rdx
	shrq	$5, %rdx
	leaq	-48(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	add_decimal
	.loc 1 749 13 discriminator 1
	testl	%eax, %eax
	je	.L277
	.loc 1 749 45 discriminator 1
	leaq	-48(%rbp), %rax
	movl	$46, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 749 41 discriminator 1
	testl	%eax, %eax
	je	.L277
	.loc 1 750 15
	movq	-72(%rbp), %rcx
	movabsq	$-3689348814741910323, %rdx
	movq	%rcx, %rax
	mulq	%rdx
	shrq	$5, %rdx
	movq	%rdx, %rax
	salq	$2, %rax
	addq	%rdx, %rax
	salq	$3, %rax
	subq	%rax, %rcx
	movq	%rcx, %rdx
	leaq	-48(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	add_decimal
	.loc 1 749 67 discriminator 2
	testl	%eax, %eax
	je	.L277
	.loc 1 754 9
	jmp	.L267
.L269:
	.loc 1 755 10
	leaq	-72(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	parse_base128_integer
	.loc 1 755 8 discriminator 1
	testl	%eax, %eax
	je	.L278
	.loc 1 755 47 discriminator 1
	leaq	-48(%rbp), %rax
	movl	$46, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 755 43 discriminator 1
	testl	%eax, %eax
	je	.L278
	.loc 1 756 10
	movq	-72(%rbp), %rdx
	leaq	-48(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	add_decimal
	.loc 1 755 69 discriminator 2
	testl	%eax, %eax
	je	.L278
.L267:
	.loc 1 754 10
	leaq	-64(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 754 25 discriminator 1
	testq	%rax, %rax
	jne	.L269
	.loc 1 763 8
	leaq	-48(%rbp), %rax
	movl	$0, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 763 6 discriminator 1
	testl	%eax, %eax
	je	.L279
	.loc 1 763 35 discriminator 1
	leaq	-88(%rbp), %rdx
	leaq	-80(%rbp), %rcx
	leaq	-48(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_finish@PLT
	.loc 1 763 31 discriminator 1
	testl	%eax, %eax
	je	.L279
	.loc 1 767 10
	movq	-80(%rbp), %rax
	jmp	.L273
.L274:
	.loc 1 734 5
	nop
	jmp	.L261
.L275:
	.loc 1 741 5
	nop
	jmp	.L261
.L276:
	.loc 1 747 7
	nop
	jmp	.L261
.L277:
	.loc 1 751 5
	nop
	jmp	.L261
.L278:
	.loc 1 757 7
	nop
	jmp	.L261
.L279:
	.loc 1 764 5
	nop
.L261:
	.loc 1 770 3
	leaq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_cleanup@PLT
	.loc 1 771 10
	movl	$0, %eax
.L273:
	.loc 1 772 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE313:
	.size	aws_lc_0_38_0_CBS_asn1_oid_to_text, .-aws_lc_0_38_0_CBS_asn1_oid_to_text
	.section	.text.cbs_get_two_digits,"ax",@progbits
	.type	cbs_get_two_digits, @function
cbs_get_two_digits:
.LFB314:
	.loc 1 774 51
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 776 8
	leaq	-1(%rbp), %rdx
	movq	-24(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 776 6 discriminator 1
	testl	%eax, %eax
	jne	.L281
	.loc 1 777 12
	movl	$0, %eax
	jmp	.L286
.L281:
	.loc 1 779 8
	movzbl	-1(%rbp), %eax
	movzbl	%al, %eax
	movl	%eax, %edi
	call	aws_lc_0_38_0_OPENSSL_isdigit@PLT
	.loc 1 779 6 discriminator 1
	testl	%eax, %eax
	jne	.L283
	.loc 1 780 12
	movl	$0, %eax
	jmp	.L286
.L283:
	.loc 1 782 8
	leaq	-2(%rbp), %rdx
	movq	-24(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 782 6 discriminator 1
	testl	%eax, %eax
	jne	.L284
	.loc 1 783 12
	movl	$0, %eax
	jmp	.L286
.L284:
	.loc 1 785 8
	movzbl	-2(%rbp), %eax
	movzbl	%al, %eax
	movl	%eax, %edi
	call	aws_lc_0_38_0_OPENSSL_isdigit@PLT
	.loc 1 785 6 discriminator 1
	testl	%eax, %eax
	jne	.L285
	.loc 1 786 12
	movl	$0, %eax
	jmp	.L286
.L285:
	.loc 1 788 23
	movzbl	-1(%rbp), %eax
	movzbl	%al, %eax
	leal	-48(%rax), %edx
	.loc 1 788 30
	movl	%edx, %eax
	sall	$2, %eax
	addl	%edx, %eax
	addl	%eax, %eax
	movl	%eax, %edx
	.loc 1 788 51
	movzbl	-2(%rbp), %eax
	movzbl	%al, %eax
	subl	$48, %eax
	.loc 1 788 35
	addl	%eax, %edx
	.loc 1 788 8
	movq	-32(%rbp), %rax
	movl	%edx, (%rax)
	.loc 1 789 10
	movl	$1, %eax
.L286:
	.loc 1 790 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE314:
	.size	cbs_get_two_digits, .-cbs_get_two_digits
	.section	.text.is_valid_day,"ax",@progbits
	.type	is_valid_day, @function
is_valid_day:
.LFB315:
	.loc 1 792 55
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movl	%edi, -4(%rbp)
	movl	%esi, -8(%rbp)
	movl	%edx, -12(%rbp)
	.loc 1 793 6
	cmpl	$0, -12(%rbp)
	jg	.L288
	.loc 1 794 12
	movl	$0, %eax
	jmp	.L289
.L288:
	.loc 1 796 3
	movl	-8(%rbp), %eax
	cmpl	$12, %eax
	seta	%dl
	testb	%dl, %dl
	jne	.L290
	movl	$1, %edx
	shlx	%rax, %rdx, %rax
	movq	%rax, %rdx
	andl	$5546, %edx
	testq	%rdx, %rdx
	setne	%dl
	testb	%dl, %dl
	jne	.L291
	movq	%rax, %rdx
	andl	$2640, %edx
	testq	%rdx, %rdx
	setne	%dl
	testb	%dl, %dl
	jne	.L292
	andl	$4, %eax
	testq	%rax, %rax
	setne	%al
	testb	%al, %al
	jne	.L293
	jmp	.L290
.L291:
	.loc 1 804 18
	cmpl	$31, -12(%rbp)
	setle	%al
	movzbl	%al, %eax
	jmp	.L289
.L292:
	.loc 1 809 18
	cmpl	$30, -12(%rbp)
	setle	%al
	movzbl	%al, %eax
	jmp	.L289
.L293:
	.loc 1 811 21
	movl	-4(%rbp), %eax
	andl	$3, %eax
	.loc 1 811 10
	testl	%eax, %eax
	jne	.L294
	.loc 1 811 34 discriminator 1
	movl	-4(%rbp), %edx
	movslq	%edx, %rax
	imulq	$1374389535, %rax, %rax
	shrq	$32, %rax
	sarl	$5, %eax
	movl	%edx, %ecx
	sarl	$31, %ecx
	subl	%ecx, %eax
	imull	$100, %eax, %ecx
	movl	%edx, %eax
	subl	%ecx, %eax
	.loc 1 811 26 discriminator 1
	testl	%eax, %eax
	jne	.L295
.L294:
	.loc 1 811 54 discriminator 3
	movl	-4(%rbp), %edx
	movslq	%edx, %rax
	imulq	$1374389535, %rax, %rax
	shrq	$32, %rax
	sarl	$7, %eax
	movl	%edx, %ecx
	sarl	$31, %ecx
	subl	%ecx, %eax
	imull	$400, %eax, %ecx
	movl	%edx, %eax
	subl	%ecx, %eax
	.loc 1 811 46 discriminator 3
	testl	%eax, %eax
	jne	.L296
.L295:
	.loc 1 812 20
	cmpl	$29, -12(%rbp)
	setle	%al
	movzbl	%al, %eax
	jmp	.L289
.L296:
	.loc 1 814 20
	cmpl	$28, -12(%rbp)
	setle	%al
	movzbl	%al, %eax
	jmp	.L289
.L290:
	.loc 1 817 14
	movl	$0, %eax
.L289:
	.loc 1 819 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE315:
	.size	is_valid_day, .-is_valid_day
	.section	.text.CBS_parse_rfc5280_time_internal,"ax",@progbits
	.type	CBS_parse_rfc5280_time_internal, @function
CBS_parse_rfc5280_time_internal:
.LFB316:
	.loc 1 823 63
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$112, %rsp
	movq	%rdi, -88(%rbp)
	movl	%esi, -92(%rbp)
	movl	%edx, -96(%rbp)
	movq	%rcx, -104(%rbp)
	.loc 1 825 7
	movq	-88(%rbp), %rax
	movq	8(%rax), %rdx
	movq	(%rax), %rax
	movq	%rax, -64(%rbp)
	movq	%rdx, -56(%rbp)
	.loc 1 828 6
	cmpl	$0, -92(%rbp)
	je	.L298
	.loc 1 829 10
	leaq	-36(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	cbs_get_two_digits
	.loc 1 829 8 discriminator 1
	testl	%eax, %eax
	jne	.L299
	.loc 1 830 14
	movl	$0, %eax
	jmp	.L319
.L299:
	.loc 1 832 16
	movl	-36(%rbp), %eax
	.loc 1 832 10
	imull	$100, %eax, %eax
	movl	%eax, -4(%rbp)
	.loc 1 833 10
	leaq	-36(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	cbs_get_two_digits
	.loc 1 833 8 discriminator 1
	testl	%eax, %eax
	jne	.L301
	.loc 1 834 14
	movl	$0, %eax
	jmp	.L319
.L301:
	.loc 1 836 10
	movl	-36(%rbp), %eax
	addl	%eax, -4(%rbp)
	jmp	.L302
.L298:
	.loc 1 838 10
	movl	$1900, -4(%rbp)
	.loc 1 839 10
	leaq	-36(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	cbs_get_two_digits
	.loc 1 839 8 discriminator 1
	testl	%eax, %eax
	jne	.L303
	.loc 1 840 14
	movl	$0, %eax
	jmp	.L319
.L303:
	.loc 1 842 10
	movl	-36(%rbp), %eax
	addl	%eax, -4(%rbp)
	.loc 1 843 8
	cmpl	$1949, -4(%rbp)
	jg	.L304
	.loc 1 844 12
	addl	$100, -4(%rbp)
.L304:
	.loc 1 846 8
	cmpl	$2049, -4(%rbp)
	jle	.L302
	.loc 1 847 14
	movl	$0, %eax
	jmp	.L319
.L302:
	.loc 1 850 8
	leaq	-16(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	cbs_get_two_digits
	.loc 1 850 6 discriminator 1
	testl	%eax, %eax
	je	.L305
	.loc 1 850 51 discriminator 1
	movl	-16(%rbp), %eax
	.loc 1 850 42 discriminator 1
	testl	%eax, %eax
	jle	.L305
	.loc 1 851 13
	movl	-16(%rbp), %eax
	.loc 1 850 55 discriminator 2
	cmpl	$12, %eax
	jg	.L305
	.loc 1 852 8
	leaq	-20(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	cbs_get_two_digits
	.loc 1 851 18
	testl	%eax, %eax
	je	.L305
	.loc 1 853 8
	movl	-20(%rbp), %edx
	movl	-16(%rbp), %ecx
	movl	-4(%rbp), %eax
	movl	%ecx, %esi
	movl	%eax, %edi
	call	is_valid_day
	.loc 1 852 40
	testl	%eax, %eax
	je	.L305
	.loc 1 854 8
	leaq	-24(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	cbs_get_two_digits
	.loc 1 853 39
	testl	%eax, %eax
	je	.L305
	.loc 1 855 12
	movl	-24(%rbp), %eax
	.loc 1 854 41
	cmpl	$23, %eax
	jg	.L305
	.loc 1 856 8
	leaq	-28(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	cbs_get_two_digits
	.loc 1 855 17
	testl	%eax, %eax
	je	.L305
	.loc 1 857 11
	movl	-28(%rbp), %eax
	.loc 1 856 40
	cmpl	$59, %eax
	jg	.L305
	.loc 1 858 8
	leaq	-32(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	cbs_get_two_digits
	.loc 1 857 16
	testl	%eax, %eax
	je	.L305
	.loc 1 858 47
	movl	-32(%rbp), %eax
	.loc 1 858 40
	cmpl	$59, %eax
	jg	.L305
	.loc 1 858 56 discriminator 1
	leaq	-65(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 858 52 discriminator 1
	testl	%eax, %eax
	jne	.L306
.L305:
	.loc 1 859 12
	movl	$0, %eax
	jmp	.L319
.L306:
	.loc 1 862 7
	movl	$0, -8(%rbp)
	.loc 1 863 3
	movzbl	-65(%rbp), %eax
	movzbl	%al, %eax
	cmpl	$90, %eax
	je	.L320
	cmpl	$90, %eax
	jg	.L308
	cmpl	$43, %eax
	je	.L309
	cmpl	$45, %eax
	je	.L310
	jmp	.L308
.L309:
	.loc 1 867 19
	movl	$1, -8(%rbp)
	.loc 1 868 7
	jmp	.L311
.L310:
	.loc 1 870 19
	movl	$-1, -8(%rbp)
	.loc 1 871 7
	jmp	.L311
.L308:
	.loc 1 873 14
	movl	$0, %eax
	jmp	.L319
.L320:
	.loc 1 865 7
	nop
.L311:
	.loc 1 888 7
	movl	$0, -12(%rbp)
	.loc 1 889 6
	cmpl	$0, -8(%rbp)
	je	.L312
.LBB10:
	.loc 1 890 8
	cmpl	$0, -96(%rbp)
	jne	.L313
	.loc 1 891 14
	movl	$0, %eax
	jmp	.L319
.L313:
	.loc 1 894 10
	leaq	-72(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	cbs_get_two_digits
	.loc 1 894 8 discriminator 1
	testl	%eax, %eax
	je	.L315
	.loc 1 895 22
	movl	-72(%rbp), %eax
	.loc 1 894 51 discriminator 1
	cmpl	$23, %eax
	jg	.L315
	.loc 1 896 10
	leaq	-76(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	cbs_get_two_digits
	.loc 1 895 27
	testl	%eax, %eax
	je	.L315
	.loc 1 897 24
	movl	-76(%rbp), %eax
	.loc 1 896 53
	cmpl	$59, %eax
	jle	.L316
.L315:
	.loc 1 898 14
	movl	$0, %eax
	jmp	.L319
.L316:
	.loc 1 900 50
	movl	-72(%rbp), %eax
	imull	$3600, %eax, %edx
	.loc 1 900 74
	movl	-76(%rbp), %eax
	imull	$60, %eax, %eax
	.loc 1 900 57
	addl	%eax, %edx
	.loc 1 900 20
	movl	-8(%rbp), %eax
	imull	%edx, %eax
	movl	%eax, -12(%rbp)
.L312:
.LBE10:
	.loc 1 903 7
	leaq	-64(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 903 6 discriminator 1
	testq	%rax, %rax
	je	.L317
	.loc 1 904 12
	movl	$0, %eax
	jmp	.L319
.L317:
	.loc 1 907 6
	cmpq	$0, -104(%rbp)
	je	.L318
	.loc 1 909 28
	movl	-4(%rbp), %eax
	leal	-1900(%rax), %edx
	.loc 1 909 21
	movq	-104(%rbp), %rax
	movl	%edx, 20(%rax)
	.loc 1 910 28
	movl	-16(%rbp), %eax
	leal	-1(%rax), %edx
	.loc 1 910 20
	movq	-104(%rbp), %rax
	movl	%edx, 16(%rax)
	.loc 1 911 21
	movl	-20(%rbp), %edx
	movq	-104(%rbp), %rax
	movl	%edx, 12(%rax)
	.loc 1 912 21
	movl	-24(%rbp), %edx
	movq	-104(%rbp), %rax
	movl	%edx, 8(%rax)
	.loc 1 913 20
	movl	-28(%rbp), %edx
	movq	-104(%rbp), %rax
	movl	%edx, 4(%rax)
	.loc 1 914 20
	movl	-32(%rbp), %edx
	movq	-104(%rbp), %rax
	movl	%edx, (%rax)
	.loc 1 915 8
	cmpl	$0, -12(%rbp)
	je	.L318
	.loc 1 915 28 discriminator 1
	movl	-12(%rbp), %eax
	movslq	%eax, %rdx
	movq	-104(%rbp), %rax
	movl	$0, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_gmtime_adj@PLT
	.loc 1 915 24 discriminator 1
	testl	%eax, %eax
	jne	.L318
	.loc 1 916 14
	movl	$0, %eax
	jmp	.L319
.L318:
	.loc 1 919 10
	movl	$1, %eax
.L319:
	.loc 1 920 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE316:
	.size	CBS_parse_rfc5280_time_internal, .-CBS_parse_rfc5280_time_internal
	.section	.text.aws_lc_0_38_0_CBS_parse_generalized_time,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_parse_generalized_time
	.type	aws_lc_0_38_0_CBS_parse_generalized_time, @function
aws_lc_0_38_0_CBS_parse_generalized_time:
.LFB317:
	.loc 1 923 59
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movl	%edx, -20(%rbp)
	.loc 1 924 10
	movq	-16(%rbp), %rcx
	movl	-20(%rbp), %edx
	movq	-8(%rbp), %rax
	movl	$1, %esi
	movq	%rax, %rdi
	call	CBS_parse_rfc5280_time_internal
	.loc 1 925 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE317:
	.size	aws_lc_0_38_0_CBS_parse_generalized_time, .-aws_lc_0_38_0_CBS_parse_generalized_time
	.section	.text.aws_lc_0_38_0_CBS_parse_utc_time,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_parse_utc_time
	.type	aws_lc_0_38_0_CBS_parse_utc_time, @function
aws_lc_0_38_0_CBS_parse_utc_time:
.LFB318:
	.loc 1 928 51
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movl	%edx, -20(%rbp)
	.loc 1 929 10
	movq	-16(%rbp), %rcx
	movl	-20(%rbp), %edx
	movq	-8(%rbp), %rax
	movl	$0, %esi
	movq	%rax, %rdi
	call	CBS_parse_rfc5280_time_internal
	.loc 1 930 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE318:
	.size	aws_lc_0_38_0_CBS_parse_utc_time, .-aws_lc_0_38_0_CBS_parse_utc_time
	.section	.text.aws_lc_0_38_0_CBS_get_optional_asn1_int64,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_optional_asn1_int64
	.type	aws_lc_0_38_0_CBS_get_optional_asn1_int64, @function
aws_lc_0_38_0_CBS_get_optional_asn1_int64:
.LFB319:
	.loc 1 933 56
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	movl	%edx, -52(%rbp)
	movq	%rcx, -64(%rbp)
	.loc 1 936 8
	movl	-52(%rbp), %ecx
	leaq	-20(%rbp), %rdx
	leaq	-16(%rbp), %rsi
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_optional_asn1@PLT
	.loc 1 936 6 discriminator 1
	testl	%eax, %eax
	jne	.L326
	.loc 1 937 12
	movl	$0, %eax
	jmp	.L331
.L326:
	.loc 1 939 7
	movl	-20(%rbp), %eax
	.loc 1 939 6
	testl	%eax, %eax
	je	.L328
	.loc 1 940 10
	movq	-48(%rbp), %rdx
	leaq	-16(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1_int64@PLT
	.loc 1 940 8 discriminator 1
	testl	%eax, %eax
	je	.L329
	.loc 1 940 45 discriminator 1
	leaq	-16(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 940 42 discriminator 1
	testq	%rax, %rax
	je	.L330
.L329:
	.loc 1 941 14
	movl	$0, %eax
	jmp	.L331
.L328:
	.loc 1 944 10
	movq	-48(%rbp), %rax
	movq	-64(%rbp), %rdx
	movq	%rdx, (%rax)
.L330:
	.loc 1 946 10
	movl	$1, %eax
.L331:
	.loc 1 947 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE319:
	.size	aws_lc_0_38_0_CBS_get_optional_asn1_int64, .-aws_lc_0_38_0_CBS_get_optional_asn1_int64
	.section	.rodata.__PRETTY_FUNCTION__.4,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.4, @object
	.size	__PRETTY_FUNCTION__.4, 24
__PRETTY_FUNCTION__.4:
	.string	"cbs_get_length_prefixed"
	.section	.rodata.__PRETTY_FUNCTION__.3,"a"
	.align 32
	.type	__PRETTY_FUNCTION__.3, @object
	.size	__PRETTY_FUNCTION__.3, 39
__PRETTY_FUNCTION__.3:
	.string	"aws_lc_0_38_0_cbs_get_any_asn1_element"
	.section	.rodata.__PRETTY_FUNCTION__.2,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.2, @object
	.size	__PRETTY_FUNCTION__.2, 31
__PRETTY_FUNCTION__.2:
	.string	"aws_lc_0_38_0_CBS_get_any_asn1"
	.section	.rodata.__PRETTY_FUNCTION__.1,"a"
	.align 8
	.type	__PRETTY_FUNCTION__.1, @object
	.size	__PRETTY_FUNCTION__.1, 13
__PRETTY_FUNCTION__.1:
	.string	"cbs_get_asn1"
	.section	.rodata.__PRETTY_FUNCTION__.0,"a"
	.align 32
	.type	__PRETTY_FUNCTION__.0, @object
	.size	__PRETTY_FUNCTION__.0, 49
__PRETTY_FUNCTION__.0:
	.string	"aws_lc_0_38_0_CBS_get_optional_asn1_octet_string"
	.text
.Letext0:
	.file 3 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 4 "/usr/include/bits/types.h"
	.file 5 "/usr/include/bits/stdint-intn.h"
	.file 6 "/usr/include/bits/stdint-uintn.h"
	.file 7 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.file 8 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/bytestring.h"
	.file 9 "/usr/include/bits/types/struct_tm.h"
	.file 10 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/../asn1/internal.h"
	.file 11 "/usr/include/string.h"
	.file 12 "/usr/include/stdio.h"
	.file 13 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/mem.h"
	.file 14 "/usr/include/assert.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x1ede
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF185
	.byte	0xc
	.long	.LASF186
	.long	.LASF187
	.long	.Ldebug_ranges0+0x30
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
	.long	0x41
	.uleb128 0x4
	.long	0x30
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF1
	.uleb128 0x5
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
	.uleb128 0x4
	.long	0x6b
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF7
	.uleb128 0x3
	.long	.LASF9
	.byte	0x4
	.byte	0x26
	.byte	0x17
	.long	0x5d
	.uleb128 0x2
	.byte	0x2
	.byte	0x5
	.long	.LASF10
	.uleb128 0x3
	.long	.LASF11
	.byte	0x4
	.byte	0x28
	.byte	0x1c
	.long	0x64
	.uleb128 0x3
	.long	.LASF12
	.byte	0x4
	.byte	0x2a
	.byte	0x16
	.long	0x6b
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
	.long	0x41
	.uleb128 0x6
	.byte	0x8
	.uleb128 0x7
	.byte	0x8
	.long	0xc9
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF15
	.uleb128 0x4
	.long	0xc9
	.uleb128 0x3
	.long	.LASF16
	.byte	0x5
	.byte	0x1b
	.byte	0x13
	.long	0xa9
	.uleb128 0x3
	.long	.LASF17
	.byte	0x6
	.byte	0x18
	.byte	0x13
	.long	0x7e
	.uleb128 0x4
	.long	0xe1
	.uleb128 0x3
	.long	.LASF18
	.byte	0x6
	.byte	0x19
	.byte	0x14
	.long	0x91
	.uleb128 0x3
	.long	.LASF19
	.byte	0x6
	.byte	0x1a
	.byte	0x14
	.long	0x9d
	.uleb128 0x3
	.long	.LASF20
	.byte	0x6
	.byte	0x1b
	.byte	0x14
	.long	0xb5
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF21
	.uleb128 0x7
	.byte	0x8
	.long	0x123
	.uleb128 0x8
	.uleb128 0x9
	.long	.LASF22
	.byte	0x7
	.value	0x158
	.byte	0x12
	.long	0xfe
	.uleb128 0xa
	.string	"CBB"
	.byte	0x7
	.value	0x194
	.byte	0x17
	.long	0x13e
	.uleb128 0xb
	.long	.LASF25
	.byte	0x30
	.byte	0x8
	.value	0x1be
	.byte	0x8
	.long	0x175
	.uleb128 0xc
	.long	.LASF23
	.byte	0x8
	.value	0x1c0
	.byte	0x8
	.long	0x33a
	.byte	0
	.uleb128 0xc
	.long	.LASF24
	.byte	0x8
	.value	0x1c3
	.byte	0x8
	.long	0xc9
	.byte	0x8
	.uleb128 0xd
	.string	"u"
	.byte	0x8
	.value	0x1c7
	.byte	0x5
	.long	0x315
	.byte	0x10
	.byte	0
	.uleb128 0xa
	.string	"CBS"
	.byte	0x7
	.value	0x195
	.byte	0x17
	.long	0x187
	.uleb128 0x4
	.long	0x175
	.uleb128 0xe
	.long	.LASF26
	.byte	0x10
	.byte	0x8
	.byte	0x28
	.byte	0x8
	.long	0x1af
	.uleb128 0xf
	.long	.LASF27
	.byte	0x8
	.byte	0x29
	.byte	0x12
	.long	0x25e
	.byte	0
	.uleb128 0x10
	.string	"len"
	.byte	0x8
	.byte	0x2a
	.byte	0xa
	.long	0x30
	.byte	0x8
	.byte	0
	.uleb128 0x11
	.string	"tm"
	.byte	0x38
	.byte	0x9
	.byte	0x7
	.byte	0x8
	.long	0x24b
	.uleb128 0xf
	.long	.LASF28
	.byte	0x9
	.byte	0x9
	.byte	0x7
	.long	0x48
	.byte	0
	.uleb128 0xf
	.long	.LASF29
	.byte	0x9
	.byte	0xa
	.byte	0x7
	.long	0x48
	.byte	0x4
	.uleb128 0xf
	.long	.LASF30
	.byte	0x9
	.byte	0xb
	.byte	0x7
	.long	0x48
	.byte	0x8
	.uleb128 0xf
	.long	.LASF31
	.byte	0x9
	.byte	0xc
	.byte	0x7
	.long	0x48
	.byte	0xc
	.uleb128 0xf
	.long	.LASF32
	.byte	0x9
	.byte	0xd
	.byte	0x7
	.long	0x48
	.byte	0x10
	.uleb128 0xf
	.long	.LASF33
	.byte	0x9
	.byte	0xe
	.byte	0x7
	.long	0x48
	.byte	0x14
	.uleb128 0xf
	.long	.LASF34
	.byte	0x9
	.byte	0xf
	.byte	0x7
	.long	0x48
	.byte	0x18
	.uleb128 0xf
	.long	.LASF35
	.byte	0x9
	.byte	0x10
	.byte	0x7
	.long	0x48
	.byte	0x1c
	.uleb128 0xf
	.long	.LASF36
	.byte	0x9
	.byte	0x11
	.byte	0x7
	.long	0x48
	.byte	0x20
	.uleb128 0xf
	.long	.LASF37
	.byte	0x9
	.byte	0x17
	.byte	0xc
	.long	0x29
	.byte	0x28
	.uleb128 0xf
	.long	.LASF38
	.byte	0x9
	.byte	0x18
	.byte	0xf
	.long	0x24b
	.byte	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xd0
	.uleb128 0x2
	.byte	0x8
	.byte	0x4
	.long	.LASF39
	.uleb128 0x7
	.byte	0x8
	.long	0x30
	.uleb128 0x7
	.byte	0x8
	.long	0xed
	.uleb128 0xb
	.long	.LASF40
	.byte	0x20
	.byte	0x8
	.value	0x1a4
	.byte	0x8
	.long	0x2bf
	.uleb128 0xd
	.string	"buf"
	.byte	0x8
	.value	0x1a5
	.byte	0xc
	.long	0x2bf
	.byte	0
	.uleb128 0xd
	.string	"len"
	.byte	0x8
	.value	0x1a7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0xd
	.string	"cap"
	.byte	0x8
	.value	0x1a9
	.byte	0xa
	.long	0x30
	.byte	0x10
	.uleb128 0x12
	.long	.LASF41
	.byte	0x8
	.value	0x1ac
	.byte	0xc
	.long	0x6b
	.byte	0x4
	.byte	0x1
	.byte	0x1f
	.byte	0x18
	.uleb128 0x12
	.long	.LASF42
	.byte	0x8
	.value	0x1af
	.byte	0xc
	.long	0x6b
	.byte	0x4
	.byte	0x1
	.byte	0x1e
	.byte	0x18
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xe1
	.uleb128 0xb
	.long	.LASF43
	.byte	0x18
	.byte	0x8
	.value	0x1b2
	.byte	0x8
	.long	0x30f
	.uleb128 0xc
	.long	.LASF44
	.byte	0x8
	.value	0x1b4
	.byte	0x19
	.long	0x30f
	.byte	0
	.uleb128 0xc
	.long	.LASF45
	.byte	0x8
	.value	0x1b7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0xc
	.long	.LASF46
	.byte	0x8
	.value	0x1ba
	.byte	0xb
	.long	0xe1
	.byte	0x10
	.uleb128 0x12
	.long	.LASF47
	.byte	0x8
	.value	0x1bb
	.byte	0xc
	.long	0x6b
	.byte	0x4
	.byte	0x1
	.byte	0x17
	.byte	0x10
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x264
	.uleb128 0x13
	.byte	0x20
	.byte	0x8
	.value	0x1c4
	.byte	0x3
	.long	0x33a
	.uleb128 0x14
	.long	.LASF44
	.byte	0x8
	.value	0x1c5
	.byte	0x1a
	.long	0x264
	.uleb128 0x14
	.long	.LASF23
	.byte	0x8
	.value	0x1c6
	.byte	0x19
	.long	0x2c5
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x131
	.uleb128 0x2
	.byte	0x10
	.byte	0x5
	.long	.LASF48
	.uleb128 0x2
	.byte	0x10
	.byte	0x7
	.long	.LASF49
	.uleb128 0x2
	.byte	0x1
	.byte	0x2
	.long	.LASF50
	.uleb128 0x15
	.long	.LASF51
	.byte	0xa
	.byte	0x56
	.byte	0x14
	.long	0x48
	.long	0x375
	.uleb128 0x16
	.long	0x375
	.uleb128 0x16
	.long	0x48
	.uleb128 0x16
	.long	0xd5
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x1af
	.uleb128 0x17
	.long	.LASF66
	.byte	0x8
	.value	0x1e2
	.byte	0x15
	.long	0x38e
	.uleb128 0x16
	.long	0x33a
	.byte	0
	.uleb128 0x18
	.long	.LASF52
	.byte	0x8
	.value	0x1ec
	.byte	0x14
	.long	0x48
	.long	0x3af
	.uleb128 0x16
	.long	0x33a
	.uleb128 0x16
	.long	0x3af
	.uleb128 0x16
	.long	0x258
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x2bf
	.uleb128 0x18
	.long	.LASF53
	.byte	0x8
	.value	0x232
	.byte	0x14
	.long	0x48
	.long	0x3d1
	.uleb128 0x16
	.long	0x33a
	.uleb128 0x16
	.long	0xe1
	.byte	0
	.uleb128 0x18
	.long	.LASF54
	.byte	0x8
	.value	0x1d3
	.byte	0x14
	.long	0x48
	.long	0x3ed
	.uleb128 0x16
	.long	0x33a
	.uleb128 0x16
	.long	0x30
	.byte	0
	.uleb128 0x18
	.long	.LASF55
	.byte	0x8
	.value	0x219
	.byte	0x14
	.long	0x48
	.long	0x40e
	.uleb128 0x16
	.long	0x33a
	.uleb128 0x16
	.long	0x25e
	.uleb128 0x16
	.long	0x30
	.byte	0
	.uleb128 0x18
	.long	.LASF56
	.byte	0xb
	.value	0x197
	.byte	0xf
	.long	0x30
	.long	0x425
	.uleb128 0x16
	.long	0x24b
	.byte	0
	.uleb128 0x18
	.long	.LASF57
	.byte	0xc
	.value	0x181
	.byte	0xc
	.long	0x48
	.long	0x447
	.uleb128 0x16
	.long	0xc3
	.uleb128 0x16
	.long	0x41
	.uleb128 0x16
	.long	0x24b
	.uleb128 0x19
	.byte	0
	.uleb128 0x15
	.long	.LASF58
	.byte	0xb
	.byte	0x3d
	.byte	0xe
	.long	0xc1
	.long	0x467
	.uleb128 0x16
	.long	0xc1
	.uleb128 0x16
	.long	0x48
	.uleb128 0x16
	.long	0x41
	.byte	0
	.uleb128 0x15
	.long	.LASF59
	.byte	0xd
	.byte	0x88
	.byte	0x14
	.long	0x48
	.long	0x47d
	.uleb128 0x16
	.long	0x48
	.byte	0
	.uleb128 0x1a
	.long	.LASF60
	.byte	0xe
	.byte	0x43
	.byte	0xd
	.long	0x49e
	.uleb128 0x16
	.long	0x24b
	.uleb128 0x16
	.long	0x24b
	.uleb128 0x16
	.long	0x6b
	.uleb128 0x16
	.long	0x24b
	.byte	0
	.uleb128 0x15
	.long	.LASF61
	.byte	0xb
	.byte	0x2b
	.byte	0xe
	.long	0xc1
	.long	0x4be
	.uleb128 0x16
	.long	0xc1
	.uleb128 0x16
	.long	0x11d
	.uleb128 0x16
	.long	0x41
	.byte	0
	.uleb128 0x15
	.long	.LASF62
	.byte	0xd
	.byte	0x74
	.byte	0x14
	.long	0x48
	.long	0x4de
	.uleb128 0x16
	.long	0x11d
	.uleb128 0x16
	.long	0x11d
	.uleb128 0x16
	.long	0x30
	.byte	0
	.uleb128 0x15
	.long	.LASF63
	.byte	0xb
	.byte	0x6b
	.byte	0xe
	.long	0xc1
	.long	0x4fe
	.uleb128 0x16
	.long	0x11d
	.uleb128 0x16
	.long	0x48
	.uleb128 0x16
	.long	0x41
	.byte	0
	.uleb128 0x15
	.long	.LASF64
	.byte	0xd
	.byte	0xc9
	.byte	0x16
	.long	0xc3
	.long	0x519
	.uleb128 0x16
	.long	0x24b
	.uleb128 0x16
	.long	0x30
	.byte	0
	.uleb128 0x15
	.long	.LASF65
	.byte	0xd
	.byte	0xce
	.byte	0x16
	.long	0xc1
	.long	0x534
	.uleb128 0x16
	.long	0x11d
	.uleb128 0x16
	.long	0x30
	.byte	0
	.uleb128 0x1b
	.long	.LASF67
	.byte	0xd
	.byte	0x69
	.byte	0x15
	.long	0x546
	.uleb128 0x16
	.long	0xc1
	.byte	0
	.uleb128 0x1c
	.long	.LASF70
	.byte	0x1
	.value	0x3a4
	.byte	0x5
	.long	0x48
	.quad	.LFB319
	.quad	.LFE319-.LFB319
	.uleb128 0x1
	.byte	0x9c
	.long	0x5cc
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x3a4
	.byte	0x26
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x3a4
	.byte	0x34
	.long	0x5d2
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1d
	.string	"tag"
	.byte	0x1
	.value	0x3a4
	.byte	0x46
	.long	0x124
	.uleb128 0x3
	.byte	0x91
	.sleb128 -68
	.uleb128 0x1e
	.long	.LASF68
	.byte	0x1
	.value	0x3a5
	.byte	0x29
	.long	0xd5
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x1f
	.long	.LASF23
	.byte	0x1
	.value	0x3a6
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1f
	.long	.LASF69
	.byte	0x1
	.value	0x3a7
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x175
	.uleb128 0x7
	.byte	0x8
	.long	0xd5
	.uleb128 0x1c
	.long	.LASF71
	.byte	0x1
	.value	0x39f
	.byte	0x5
	.long	0x48
	.quad	.LFB318
	.quad	.LFE318-.LFB318
	.uleb128 0x1
	.byte	0x9c
	.long	0x62c
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x39f
	.byte	0x23
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1e
	.long	.LASF72
	.byte	0x1
	.value	0x39f
	.byte	0x33
	.long	0x375
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1e
	.long	.LASF73
	.byte	0x1
	.value	0x3a0
	.byte	0x1c
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x182
	.uleb128 0x1c
	.long	.LASF74
	.byte	0x1
	.value	0x39a
	.byte	0x5
	.long	0x48
	.quad	.LFB317
	.quad	.LFE317-.LFB317
	.uleb128 0x1
	.byte	0x9c
	.long	0x686
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x39a
	.byte	0x2b
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1e
	.long	.LASF72
	.byte	0x1
	.value	0x39a
	.byte	0x3b
	.long	0x375
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1e
	.long	.LASF73
	.byte	0x1
	.value	0x39b
	.byte	0x24
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.byte	0
	.uleb128 0x20
	.long	.LASF84
	.byte	0x1
	.value	0x335
	.byte	0xc
	.long	0x48
	.quad	.LFB316
	.quad	.LFE316-.LFB316
	.uleb128 0x1
	.byte	0x9c
	.long	0x7d3
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x335
	.byte	0x37
	.long	0x62c
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x1e
	.long	.LASF75
	.byte	0x1
	.value	0x335
	.byte	0x40
	.long	0x48
	.uleb128 0x3
	.byte	0x91
	.sleb128 -108
	.uleb128 0x1e
	.long	.LASF73
	.byte	0x1
	.value	0x336
	.byte	0x30
	.long	0x48
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x1e
	.long	.LASF72
	.byte	0x1
	.value	0x337
	.byte	0x37
	.long	0x375
	.uleb128 0x3
	.byte	0x91
	.sleb128 -120
	.uleb128 0x1f
	.long	.LASF76
	.byte	0x1
	.value	0x338
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x1f
	.long	.LASF77
	.byte	0x1
	.value	0x338
	.byte	0xd
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x21
	.string	"day"
	.byte	0x1
	.value	0x338
	.byte	0x14
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.uleb128 0x1f
	.long	.LASF78
	.byte	0x1
	.value	0x338
	.byte	0x19
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x21
	.string	"min"
	.byte	0x1
	.value	0x338
	.byte	0x1f
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -44
	.uleb128 0x21
	.string	"sec"
	.byte	0x1
	.value	0x338
	.byte	0x24
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x21
	.string	"tmp"
	.byte	0x1
	.value	0x338
	.byte	0x29
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -52
	.uleb128 0x1f
	.long	.LASF79
	.byte	0x1
	.value	0x339
	.byte	0x7
	.long	0x175
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x21
	.string	"tz"
	.byte	0x1
	.value	0x33a
	.byte	0xb
	.long	0xe1
	.uleb128 0x3
	.byte	0x91
	.sleb128 -81
	.uleb128 0x1f
	.long	.LASF80
	.byte	0x1
	.value	0x35e
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1f
	.long	.LASF81
	.byte	0x1
	.value	0x378
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.uleb128 0x22
	.quad	.LBB10
	.quad	.LBE10-.LBB10
	.uleb128 0x1f
	.long	.LASF82
	.byte	0x1
	.value	0x37d
	.byte	0x9
	.long	0x48
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x1f
	.long	.LASF83
	.byte	0x1
	.value	0x37d
	.byte	0x17
	.long	0x48
	.uleb128 0x3
	.byte	0x91
	.sleb128 -92
	.byte	0
	.byte	0
	.uleb128 0x23
	.long	.LASF85
	.byte	0x1
	.value	0x318
	.byte	0xc
	.long	0x48
	.quad	.LFB315
	.quad	.LFE315-.LFB315
	.uleb128 0x1
	.byte	0x9c
	.long	0x827
	.uleb128 0x1e
	.long	.LASF76
	.byte	0x1
	.value	0x318
	.byte	0x1d
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x1e
	.long	.LASF77
	.byte	0x1
	.value	0x318
	.byte	0x27
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1d
	.string	"day"
	.byte	0x1
	.value	0x318
	.byte	0x32
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x20
	.long	.LASF86
	.byte	0x1
	.value	0x306
	.byte	0xc
	.long	0x48
	.quad	.LFB314
	.quad	.LFE314-.LFB314
	.uleb128 0x1
	.byte	0x9c
	.long	0x88b
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x306
	.byte	0x24
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x306
	.byte	0x2e
	.long	0x88b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1f
	.long	.LASF87
	.byte	0x1
	.value	0x307
	.byte	0xb
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -17
	.uleb128 0x1f
	.long	.LASF88
	.byte	0x1
	.value	0x307
	.byte	0x18
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -18
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x48
	.uleb128 0x1c
	.long	.LASF89
	.byte	0x1
	.value	0x2db
	.byte	0x7
	.long	0xc3
	.quad	.LFB313
	.quad	.LFE313-.LFB313
	.uleb128 0x1
	.byte	0x9c
	.long	0x929
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x2db
	.byte	0x27
	.long	0x62c
	.uleb128 0x3
	.byte	0x91
	.sleb128 -120
	.uleb128 0x21
	.string	"cbb"
	.byte	0x1
	.value	0x2dc
	.byte	0x7
	.long	0x131
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x24
	.string	"err"
	.byte	0x1
	.value	0x301
	.byte	0x1
	.quad	.L261
	.uleb128 0x1f
	.long	.LASF79
	.byte	0x1
	.value	0x2e1
	.byte	0x7
	.long	0x175
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x21
	.string	"v"
	.byte	0x1
	.value	0x2e3
	.byte	0xc
	.long	0x10a
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x21
	.string	"txt"
	.byte	0x1
	.value	0x2f9
	.byte	0xc
	.long	0x2bf
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x1f
	.long	.LASF90
	.byte	0x1
	.value	0x2fa
	.byte	0xa
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.byte	0
	.uleb128 0x1c
	.long	.LASF91
	.byte	0x1
	.value	0x2c4
	.byte	0x5
	.long	0x48
	.quad	.LFB312
	.quad	.LFE312-.LFB312
	.uleb128 0x1
	.byte	0x9c
	.long	0x98c
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x2c4
	.byte	0x26
	.long	0x62c
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x1f
	.long	.LASF79
	.byte	0x1
	.value	0x2c9
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x21
	.string	"v"
	.byte	0x1
	.value	0x2ca
	.byte	0xb
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -49
	.uleb128 0x1f
	.long	.LASF92
	.byte	0x1
	.value	0x2ca
	.byte	0xe
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -17
	.byte	0
	.uleb128 0x20
	.long	.LASF93
	.byte	0x1
	.value	0x2be
	.byte	0xc
	.long	0x48
	.quad	.LFB311
	.quad	.LFE311-.LFB311
	.uleb128 0x1
	.byte	0x9c
	.long	0x9de
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x2be
	.byte	0x1d
	.long	0x33a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"v"
	.byte	0x1
	.value	0x2be
	.byte	0x2b
	.long	0x10a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x21
	.string	"buf"
	.byte	0x1
	.value	0x2bf
	.byte	0x8
	.long	0x9de
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.byte	0
	.uleb128 0x25
	.long	0xc9
	.long	0x9ee
	.uleb128 0x26
	.long	0x41
	.byte	0x17
	.byte	0
	.uleb128 0x1c
	.long	.LASF94
	.byte	0x1
	.value	0x2b9
	.byte	0x5
	.long	0x48
	.quad	.LFB310
	.quad	.LFE310-.LFB310
	.uleb128 0x1
	.byte	0x9c
	.long	0xa32
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x2b9
	.byte	0x2d
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1f
	.long	.LASF95
	.byte	0x1
	.value	0x2ba
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.uleb128 0x1c
	.long	.LASF96
	.byte	0x1
	.value	0x2a6
	.byte	0x5
	.long	0x48
	.quad	.LFB309
	.quad	.LFE309-.LFB309
	.uleb128 0x1
	.byte	0x9c
	.long	0xaa6
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x2a6
	.byte	0x2a
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1e
	.long	.LASF97
	.byte	0x1
	.value	0x2a6
	.byte	0x34
	.long	0x88b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1f
	.long	.LASF79
	.byte	0x1
	.value	0x2a7
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1f
	.long	.LASF98
	.byte	0x1
	.value	0x2a8
	.byte	0xb
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -33
	.uleb128 0x1f
	.long	.LASF99
	.byte	0x1
	.value	0x2a8
	.byte	0x17
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -34
	.byte	0
	.uleb128 0x1c
	.long	.LASF100
	.byte	0x1
	.value	0x297
	.byte	0x5
	.long	0x48
	.quad	.LFB308
	.quad	.LFE308-.LFB308
	.uleb128 0x1
	.byte	0x9c
	.long	0xb0a
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x297
	.byte	0x2b
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"bit"
	.byte	0x1
	.value	0x297
	.byte	0x39
	.long	0x6b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -60
	.uleb128 0x1f
	.long	.LASF101
	.byte	0x1
	.value	0x29c
	.byte	0x12
	.long	0x72
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.uleb128 0x1f
	.long	.LASF102
	.byte	0x1
	.value	0x29d
	.byte	0x12
	.long	0x72
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x1c
	.long	.LASF103
	.byte	0x1
	.value	0x282
	.byte	0x5
	.long	0x48
	.quad	.LFB307
	.quad	.LFE307-.LFB307
	.uleb128 0x1
	.byte	0x9c
	.long	0xb6d
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x282
	.byte	0x2c
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x21
	.string	"in"
	.byte	0x1
	.value	0x283
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1f
	.long	.LASF104
	.byte	0x1
	.value	0x284
	.byte	0xb
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -33
	.uleb128 0x1f
	.long	.LASF105
	.byte	0x1
	.value	0x28e
	.byte	0xb
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -34
	.byte	0
	.uleb128 0x1c
	.long	.LASF106
	.byte	0x1
	.value	0x265
	.byte	0x5
	.long	0x48
	.quad	.LFB306
	.quad	.LFE306-.LFB306
	.uleb128 0x1
	.byte	0x9c
	.long	0xc28
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x265
	.byte	0x25
	.long	0x5cc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x265
	.byte	0x2f
	.long	0x88b
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x1d
	.string	"tag"
	.byte	0x1
	.value	0x265
	.byte	0x41
	.long	0x124
	.uleb128 0x3
	.byte	0x91
	.sleb128 -100
	.uleb128 0x1e
	.long	.LASF68
	.byte	0x1
	.value	0x266
	.byte	0x24
	.long	0x48
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x1f
	.long	.LASF23
	.byte	0x1
	.value	0x267
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1f
	.long	.LASF107
	.byte	0x1
	.value	0x267
	.byte	0xe
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1f
	.long	.LASF69
	.byte	0x1
	.value	0x268
	.byte	0x7
	.long	0x48
	.uleb128 0x3
	.byte	0x91
	.sleb128 -68
	.uleb128 0x22
	.quad	.LBB9
	.quad	.LBE9-.LBB9
	.uleb128 0x1f
	.long	.LASF108
	.byte	0x1
	.value	0x26d
	.byte	0xd
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -17
	.byte	0
	.byte	0
	.uleb128 0x1c
	.long	.LASF109
	.byte	0x1
	.value	0x254
	.byte	0x5
	.long	0x48
	.quad	.LFB305
	.quad	.LFE305-.LFB305
	.uleb128 0x1
	.byte	0x9c
	.long	0xcae
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x254
	.byte	0x27
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x254
	.byte	0x36
	.long	0xcae
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1d
	.string	"tag"
	.byte	0x1
	.value	0x254
	.byte	0x48
	.long	0x124
	.uleb128 0x3
	.byte	0x91
	.sleb128 -68
	.uleb128 0x1e
	.long	.LASF68
	.byte	0x1
	.value	0x255
	.byte	0x2b
	.long	0x10a
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x1f
	.long	.LASF23
	.byte	0x1
	.value	0x256
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1f
	.long	.LASF69
	.byte	0x1
	.value	0x257
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x10a
	.uleb128 0x1c
	.long	.LASF110
	.byte	0x1
	.value	0x23e
	.byte	0x5
	.long	0x48
	.quad	.LFB304
	.quad	.LFE304-.LFB304
	.uleb128 0x1
	.byte	0x9c
	.long	0xd4d
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x23e
	.byte	0x2d
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x23e
	.byte	0x37
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1e
	.long	.LASF111
	.byte	0x1
	.value	0x23e
	.byte	0x41
	.long	0x88b
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x1d
	.string	"tag"
	.byte	0x1
	.value	0x23f
	.byte	0x35
	.long	0x124
	.uleb128 0x3
	.byte	0x91
	.sleb128 -76
	.uleb128 0x1f
	.long	.LASF23
	.byte	0x1
	.value	0x240
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1f
	.long	.LASF69
	.byte	0x1
	.value	0x241
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.uleb128 0x27
	.long	.LASF128
	.long	0xd5d
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.0
	.byte	0
	.uleb128 0x25
	.long	0xd0
	.long	0xd5d
	.uleb128 0x26
	.long	0x41
	.byte	0x30
	.byte	0
	.uleb128 0x4
	.long	0xd4d
	.uleb128 0x1c
	.long	.LASF112
	.byte	0x1
	.value	0x22c
	.byte	0x5
	.long	0x48
	.quad	.LFB303
	.quad	.LFE303-.LFB303
	.uleb128 0x1
	.byte	0x9c
	.long	0xdd6
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x22c
	.byte	0x20
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x22c
	.byte	0x2a
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1e
	.long	.LASF111
	.byte	0x1
	.value	0x22c
	.byte	0x34
	.long	0x88b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"tag"
	.byte	0x1
	.value	0x22d
	.byte	0x28
	.long	0x124
	.uleb128 0x2
	.byte	0x91
	.sleb128 -60
	.uleb128 0x1f
	.long	.LASF69
	.byte	0x1
	.value	0x22e
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.uleb128 0x1c
	.long	.LASF113
	.byte	0x1
	.value	0x21d
	.byte	0x5
	.long	0x48
	.quad	.LFB302
	.quad	.LFE302-.LFB302
	.uleb128 0x1
	.byte	0x9c
	.long	0xe3a
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x21d
	.byte	0x1c
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x21d
	.byte	0x26
	.long	0x88b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1f
	.long	.LASF114
	.byte	0x1
	.value	0x21e
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1f
	.long	.LASF115
	.byte	0x1
	.value	0x223
	.byte	0x11
	.long	0xed
	.uleb128 0x2
	.byte	0x91
	.sleb128 -17
	.byte	0
	.uleb128 0x1c
	.long	.LASF116
	.byte	0x1
	.value	0x1fd
	.byte	0x5
	.long	0x48
	.quad	.LFB301
	.quad	.LFE301-.LFB301
	.uleb128 0x1
	.byte	0x9c
	.long	0xef1
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x1fd
	.byte	0x1d
	.long	0x5cc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x1fd
	.byte	0x2b
	.long	0x5d2
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x1f
	.long	.LASF95
	.byte	0x1
	.value	0x1fe
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -44
	.uleb128 0x1f
	.long	.LASF114
	.byte	0x1
	.value	0x1ff
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1f
	.long	.LASF27
	.byte	0x1
	.value	0x204
	.byte	0x12
	.long	0x25e
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x21
	.string	"len"
	.byte	0x1
	.value	0x205
	.byte	0x10
	.long	0x3c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1f
	.long	.LASF117
	.byte	0x1
	.value	0x209
	.byte	0xb
	.long	0xef1
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x22
	.quad	.LBB8
	.quad	.LBE8-.LBB8
	.uleb128 0x21
	.string	"i"
	.byte	0x1
	.value	0x20d
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.byte	0
	.uleb128 0x25
	.long	0xe1
	.long	0xf01
	.uleb128 0x26
	.long	0x41
	.byte	0x7
	.byte	0
	.uleb128 0x1c
	.long	.LASF118
	.byte	0x1
	.value	0x1e7
	.byte	0x5
	.long	0x48
	.quad	.LFB300
	.quad	.LFE300-.LFB300
	.uleb128 0x1
	.byte	0x9c
	.long	0xf97
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x1e7
	.byte	0x1e
	.long	0x5cc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x1e7
	.byte	0x2d
	.long	0xcae
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x1f
	.long	.LASF114
	.byte	0x1
	.value	0x1e8
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1f
	.long	.LASF27
	.byte	0x1
	.value	0x1ef
	.byte	0x12
	.long	0x25e
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x21
	.string	"len"
	.byte	0x1
	.value	0x1f0
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x22
	.quad	.LBB7
	.quad	.LBE7-.LBB7
	.uleb128 0x21
	.string	"i"
	.byte	0x1
	.value	0x1f1
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.byte	0
	.uleb128 0x1c
	.long	.LASF119
	.byte	0x1
	.value	0x1e1
	.byte	0x5
	.long	0x48
	.quad	.LFB299
	.quad	.LFE299-.LFB299
	.uleb128 0x1
	.byte	0x9c
	.long	0xffb
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x1e1
	.byte	0x22
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1e
	.long	.LASF120
	.byte	0x1
	.value	0x1e1
	.byte	0x34
	.long	0x124
	.uleb128 0x2
	.byte	0x91
	.sleb128 -60
	.uleb128 0x1f
	.long	.LASF79
	.byte	0x1
	.value	0x1e2
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1f
	.long	.LASF121
	.byte	0x1
	.value	0x1e3
	.byte	0x10
	.long	0x124
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.byte	0
	.uleb128 0x1c
	.long	.LASF122
	.byte	0x1
	.value	0x1dd
	.byte	0x5
	.long	0x48
	.quad	.LFB298
	.quad	.LFE298-.LFB298
	.uleb128 0x1
	.byte	0x9c
	.long	0x104f
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x1dd
	.byte	0x1f
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x1dd
	.byte	0x29
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1e
	.long	.LASF120
	.byte	0x1
	.value	0x1dd
	.byte	0x3b
	.long	0x124
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.byte	0
	.uleb128 0x1c
	.long	.LASF123
	.byte	0x1
	.value	0x1d9
	.byte	0x5
	.long	0x48
	.quad	.LFB297
	.quad	.LFE297-.LFB297
	.uleb128 0x1
	.byte	0x9c
	.long	0x10a3
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x1d9
	.byte	0x17
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x1d9
	.byte	0x21
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1e
	.long	.LASF120
	.byte	0x1
	.value	0x1d9
	.byte	0x33
	.long	0x124
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.byte	0
	.uleb128 0x20
	.long	.LASF124
	.byte	0x1
	.value	0x1c2
	.byte	0xc
	.long	0x48
	.quad	.LFB296
	.quad	.LFE296-.LFB296
	.uleb128 0x1
	.byte	0x9c
	.long	0x114c
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x1c2
	.byte	0x1e
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x1c2
	.byte	0x28
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1e
	.long	.LASF120
	.byte	0x1
	.value	0x1c2
	.byte	0x3a
	.long	0x124
	.uleb128 0x3
	.byte	0x91
	.sleb128 -68
	.uleb128 0x1e
	.long	.LASF125
	.byte	0x1
	.value	0x1c3
	.byte	0x1d
	.long	0x48
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x1f
	.long	.LASF126
	.byte	0x1
	.value	0x1c4
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x21
	.string	"tag"
	.byte	0x1
	.value	0x1c5
	.byte	0x10
	.long	0x124
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.uleb128 0x1f
	.long	.LASF127
	.byte	0x1
	.value	0x1c6
	.byte	0x7
	.long	0x175
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x27
	.long	.LASF128
	.long	0x115c
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.1
	.byte	0
	.uleb128 0x25
	.long	0xd0
	.long	0x115c
	.uleb128 0x26
	.long	0x41
	.byte	0xc
	.byte	0
	.uleb128 0x4
	.long	0x114c
	.uleb128 0x1c
	.long	.LASF129
	.byte	0x1
	.value	0x1b8
	.byte	0x5
	.long	0x48
	.quad	.LFB295
	.quad	.LFE295-.LFB295
	.uleb128 0x1
	.byte	0x9c
	.long	0x11f7
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x1b8
	.byte	0x27
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x1b8
	.byte	0x31
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1e
	.long	.LASF130
	.byte	0x1
	.value	0x1b8
	.byte	0x44
	.long	0x11f7
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1e
	.long	.LASF131
	.byte	0x1
	.value	0x1b9
	.byte	0x2a
	.long	0x258
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1e
	.long	.LASF132
	.byte	0x1
	.value	0x1b9
	.byte	0x3f
	.long	0x88b
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x1e
	.long	.LASF133
	.byte	0x1
	.value	0x1ba
	.byte	0x27
	.long	0x88b
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x1f
	.long	.LASF134
	.byte	0x1
	.value	0x1bb
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x124
	.uleb128 0x1c
	.long	.LASF135
	.byte	0x1
	.value	0x1b2
	.byte	0x5
	.long	0x48
	.quad	.LFB294
	.quad	.LFE294-.LFB294
	.uleb128 0x1
	.byte	0x9c
	.long	0x1261
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x1b2
	.byte	0x23
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x1b2
	.byte	0x2d
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1e
	.long	.LASF130
	.byte	0x1
	.value	0x1b2
	.byte	0x40
	.long	0x11f7
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1e
	.long	.LASF131
	.byte	0x1
	.value	0x1b3
	.byte	0x26
	.long	0x258
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.byte	0
	.uleb128 0x1c
	.long	.LASF136
	.byte	0x1
	.value	0x1a4
	.byte	0x5
	.long	0x48
	.quad	.LFB293
	.quad	.LFE293-.LFB293
	.uleb128 0x1
	.byte	0x9c
	.long	0x12d8
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x1a4
	.byte	0x1b
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x1a4
	.byte	0x25
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1e
	.long	.LASF130
	.byte	0x1
	.value	0x1a4
	.byte	0x38
	.long	0x11f7
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1f
	.long	.LASF126
	.byte	0x1
	.value	0x1a5
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x27
	.long	.LASF128
	.long	0x12e8
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.2
	.byte	0
	.uleb128 0x25
	.long	0xd0
	.long	0x12e8
	.uleb128 0x26
	.long	0x41
	.byte	0x1e
	.byte	0
	.uleb128 0x4
	.long	0x12d8
	.uleb128 0x1c
	.long	.LASF137
	.byte	0x1
	.value	0x13f
	.byte	0x5
	.long	0x48
	.quad	.LFB292
	.quad	.LFE292-.LFB292
	.uleb128 0x1
	.byte	0x9c
	.long	0x1441
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x13f
	.byte	0x23
	.long	0x5cc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -120
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x13f
	.byte	0x2d
	.long	0x5cc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -128
	.uleb128 0x1e
	.long	.LASF130
	.byte	0x1
	.value	0x13f
	.byte	0x40
	.long	0x11f7
	.uleb128 0x3
	.byte	0x91
	.sleb128 -136
	.uleb128 0x1e
	.long	.LASF131
	.byte	0x1
	.value	0x140
	.byte	0x2d
	.long	0x258
	.uleb128 0x3
	.byte	0x91
	.sleb128 -144
	.uleb128 0x1e
	.long	.LASF132
	.byte	0x1
	.value	0x140
	.byte	0x42
	.long	0x88b
	.uleb128 0x3
	.byte	0x91
	.sleb128 -152
	.uleb128 0x1e
	.long	.LASF133
	.byte	0x1
	.value	0x141
	.byte	0x2a
	.long	0x88b
	.uleb128 0x3
	.byte	0x91
	.sleb128 -160
	.uleb128 0x1e
	.long	.LASF138
	.byte	0x1
	.value	0x141
	.byte	0x3e
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x1e
	.long	.LASF139
	.byte	0x1
	.value	0x141
	.byte	0x4a
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 8
	.uleb128 0x1f
	.long	.LASF140
	.byte	0x1
	.value	0x142
	.byte	0x7
	.long	0x175
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x1f
	.long	.LASF127
	.byte	0x1
	.value	0x143
	.byte	0x7
	.long	0x175
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x27
	.long	.LASF128
	.long	0x1451
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.3
	.uleb128 0x21
	.string	"tag"
	.byte	0x1
	.value	0x150
	.byte	0x10
	.long	0x124
	.uleb128 0x3
	.byte	0x91
	.sleb128 -100
	.uleb128 0x1f
	.long	.LASF141
	.byte	0x1
	.value	0x158
	.byte	0xb
	.long	0xe1
	.uleb128 0x3
	.byte	0x91
	.sleb128 -101
	.uleb128 0x1f
	.long	.LASF126
	.byte	0x1
	.value	0x15d
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x21
	.string	"len"
	.byte	0x1
	.value	0x15f
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x22
	.quad	.LBB6
	.quad	.LBE6-.LBB6
	.uleb128 0x1f
	.long	.LASF142
	.byte	0x1
	.value	0x16c
	.byte	0x12
	.long	0x3c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1f
	.long	.LASF143
	.byte	0x1
	.value	0x16d
	.byte	0xe
	.long	0x10a
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.byte	0
	.byte	0
	.uleb128 0x25
	.long	0xd0
	.long	0x1451
	.uleb128 0x26
	.long	0x41
	.byte	0x26
	.byte	0
	.uleb128 0x4
	.long	0x1441
	.uleb128 0x20
	.long	.LASF144
	.byte	0x1
	.value	0x118
	.byte	0xc
	.long	0x48
	.quad	.LFB291
	.quad	.LFE291-.LFB291
	.uleb128 0x1
	.byte	0x9c
	.long	0x14fb
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x118
	.byte	0x20
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x118
	.byte	0x33
	.long	0x11f7
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x1e
	.long	.LASF139
	.byte	0x1
	.value	0x118
	.byte	0x3c
	.long	0x48
	.uleb128 0x3
	.byte	0x91
	.sleb128 -68
	.uleb128 0x1f
	.long	.LASF145
	.byte	0x1
	.value	0x119
	.byte	0xb
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -25
	.uleb128 0x21
	.string	"tag"
	.byte	0x1
	.value	0x124
	.byte	0x10
	.long	0x124
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1f
	.long	.LASF146
	.byte	0x1
	.value	0x125
	.byte	0x10
	.long	0x124
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x22
	.quad	.LBB5
	.quad	.LBE5-.LBB5
	.uleb128 0x21
	.string	"v"
	.byte	0x1
	.value	0x127
	.byte	0xe
	.long	0x10a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.byte	0
	.uleb128 0x20
	.long	.LASF147
	.byte	0x1
	.value	0x100
	.byte	0xc
	.long	0x48
	.quad	.LFB290
	.quad	.LFE290-.LFB290
	.uleb128 0x1
	.byte	0x9c
	.long	0x155b
	.uleb128 0x1d
	.string	"cbs"
	.byte	0x1
	.value	0x100
	.byte	0x27
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1d
	.string	"out"
	.byte	0x1
	.value	0x100
	.byte	0x36
	.long	0xcae
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x21
	.string	"v"
	.byte	0x1
	.value	0x101
	.byte	0xc
	.long	0x10a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x21
	.string	"b"
	.byte	0x1
	.value	0x102
	.byte	0xb
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -25
	.byte	0
	.uleb128 0x28
	.long	.LASF148
	.byte	0x1
	.byte	0xe5
	.byte	0x5
	.long	0x48
	.quad	.LFB289
	.quad	.LFE289-.LFB289
	.uleb128 0x1
	.byte	0x9c
	.long	0x15cb
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xe5
	.byte	0x1e
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xe5
	.byte	0x2d
	.long	0xcae
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x2a
	.string	"v"
	.byte	0x1
	.byte	0xe6
	.byte	0xc
	.long	0x10a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2b
	.long	.LASF149
	.byte	0x1
	.byte	0xe7
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.uleb128 0x2c
	.long	.Ldebug_ranges0+0
	.uleb128 0x2a
	.string	"c"
	.byte	0x1
	.byte	0xe9
	.byte	0xd
	.long	0xe1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -29
	.byte	0
	.byte	0
	.uleb128 0x28
	.long	.LASF150
	.byte	0x1
	.byte	0xdd
	.byte	0x5
	.long	0x48
	.quad	.LFB288
	.quad	.LFE288-.LFB288
	.uleb128 0x1
	.byte	0x9c
	.long	0x1629
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xdd
	.byte	0x1e
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xdd
	.byte	0x28
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x29
	.string	"c"
	.byte	0x1
	.byte	0xdd
	.byte	0x35
	.long	0xe1
	.uleb128 0x3
	.byte	0x91
	.sleb128 -68
	.uleb128 0x2b
	.long	.LASF151
	.byte	0x1
	.byte	0xde
	.byte	0x12
	.long	0x25e
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x28
	.long	.LASF152
	.byte	0x1
	.byte	0xd9
	.byte	0x5
	.long	0x48
	.quad	.LFB287
	.quad	.LFE287-.LFB287
	.uleb128 0x1
	.byte	0x9c
	.long	0x166a
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xd9
	.byte	0x26
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xd9
	.byte	0x30
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x28
	.long	.LASF153
	.byte	0x1
	.byte	0xd5
	.byte	0x5
	.long	0x48
	.quad	.LFB286
	.quad	.LFE286-.LFB286
	.uleb128 0x1
	.byte	0x9c
	.long	0x16ab
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xd5
	.byte	0x26
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xd5
	.byte	0x30
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x28
	.long	.LASF154
	.byte	0x1
	.byte	0xd1
	.byte	0x5
	.long	0x48
	.quad	.LFB285
	.quad	.LFE285-.LFB285
	.uleb128 0x1
	.byte	0x9c
	.long	0x16ec
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xd1
	.byte	0x25
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xd1
	.byte	0x2f
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x2d
	.long	.LASF155
	.byte	0x1
	.byte	0xc6
	.byte	0xc
	.long	0x48
	.quad	.LFB284
	.quad	.LFE284-.LFB284
	.uleb128 0x1
	.byte	0x9c
	.long	0x175e
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xc6
	.byte	0x29
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xc6
	.byte	0x33
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x2e
	.long	.LASF156
	.byte	0x1
	.byte	0xc6
	.byte	0x3f
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x2a
	.string	"len"
	.byte	0x1
	.byte	0xc7
	.byte	0xc
	.long	0x10a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x27
	.long	.LASF128
	.long	0x176e
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.4
	.byte	0
	.uleb128 0x25
	.long	0xd0
	.long	0x176e
	.uleb128 0x26
	.long	0x41
	.byte	0x17
	.byte	0
	.uleb128 0x4
	.long	0x175e
	.uleb128 0x28
	.long	.LASF157
	.byte	0x1
	.byte	0xbd
	.byte	0x5
	.long	0x48
	.quad	.LFB283
	.quad	.LFE283-.LFB283
	.uleb128 0x1
	.byte	0x9c
	.long	0x17d0
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xbd
	.byte	0x19
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xbd
	.byte	0x27
	.long	0x2bf
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x29
	.string	"len"
	.byte	0x1
	.byte	0xbd
	.byte	0x33
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x2a
	.string	"v"
	.byte	0x1
	.byte	0xbe
	.byte	0x12
	.long	0x25e
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x28
	.long	.LASF158
	.byte	0x1
	.byte	0xb4
	.byte	0x5
	.long	0x48
	.quad	.LFB282
	.quad	.LFE282-.LFB282
	.uleb128 0x1
	.byte	0x9c
	.long	0x182d
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xb4
	.byte	0x18
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xb4
	.byte	0x22
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x29
	.string	"len"
	.byte	0x1
	.byte	0xb4
	.byte	0x2e
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x2a
	.string	"v"
	.byte	0x1
	.byte	0xb5
	.byte	0x12
	.long	0x25e
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x2f
	.long	.LASF159
	.byte	0x1
	.byte	0xab
	.byte	0x5
	.long	0x48
	.quad	.LFB281
	.quad	.LFE281-.LFB281
	.uleb128 0x1
	.byte	0x9c
	.long	0x186e
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xab
	.byte	0x1a
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xab
	.byte	0x28
	.long	0x2bf
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x28
	.long	.LASF160
	.byte	0x1
	.byte	0xa3
	.byte	0x5
	.long	0x48
	.quad	.LFB280
	.quad	.LFE280-.LFB280
	.uleb128 0x1
	.byte	0x9c
	.long	0x18af
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xa3
	.byte	0x18
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xa3
	.byte	0x27
	.long	0xcae
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x28
	.long	.LASF161
	.byte	0x1
	.byte	0xa1
	.byte	0x5
	.long	0x48
	.quad	.LFB279
	.quad	.LFE279-.LFB279
	.uleb128 0x1
	.byte	0x9c
	.long	0x18f0
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0xa1
	.byte	0x16
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0xa1
	.byte	0x25
	.long	0xcae
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x28
	.long	.LASF162
	.byte	0x1
	.byte	0x99
	.byte	0x5
	.long	0x48
	.quad	.LFB278
	.quad	.LFE278-.LFB278
	.uleb128 0x1
	.byte	0x9c
	.long	0x1931
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x99
	.byte	0x18
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0x99
	.byte	0x27
	.long	0x1931
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xfe
	.uleb128 0x28
	.long	.LASF163
	.byte	0x1
	.byte	0x90
	.byte	0x5
	.long	0x48
	.quad	.LFB277
	.quad	.LFE277-.LFB277
	.uleb128 0x1
	.byte	0x9c
	.long	0x1985
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x90
	.byte	0x16
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0x90
	.byte	0x25
	.long	0x1931
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x2a
	.string	"v"
	.byte	0x1
	.byte	0x91
	.byte	0xc
	.long	0x10a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x28
	.long	.LASF164
	.byte	0x1
	.byte	0x87
	.byte	0x5
	.long	0x48
	.quad	.LFB276
	.quad	.LFE276-.LFB276
	.uleb128 0x1
	.byte	0x9c
	.long	0x19d3
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x87
	.byte	0x16
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0x87
	.byte	0x25
	.long	0x1931
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x2a
	.string	"v"
	.byte	0x1
	.byte	0x88
	.byte	0xc
	.long	0x10a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x28
	.long	.LASF165
	.byte	0x1
	.byte	0x7f
	.byte	0x5
	.long	0x48
	.quad	.LFB275
	.quad	.LFE275-.LFB275
	.uleb128 0x1
	.byte	0x9c
	.long	0x1a14
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x7f
	.byte	0x18
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0x7f
	.byte	0x27
	.long	0x1a14
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xf2
	.uleb128 0x28
	.long	.LASF166
	.byte	0x1
	.byte	0x76
	.byte	0x5
	.long	0x48
	.quad	.LFB274
	.quad	.LFE274-.LFB274
	.uleb128 0x1
	.byte	0x9c
	.long	0x1a68
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x76
	.byte	0x16
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0x76
	.byte	0x25
	.long	0x1a14
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x2a
	.string	"v"
	.byte	0x1
	.byte	0x77
	.byte	0xc
	.long	0x10a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x28
	.long	.LASF167
	.byte	0x1
	.byte	0x6d
	.byte	0x5
	.long	0x48
	.quad	.LFB273
	.quad	.LFE273-.LFB273
	.uleb128 0x1
	.byte	0x9c
	.long	0x1ab6
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x6d
	.byte	0x15
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0x6d
	.byte	0x23
	.long	0x2bf
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x2a
	.string	"v"
	.byte	0x1
	.byte	0x6e
	.byte	0x12
	.long	0x25e
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x2d
	.long	.LASF168
	.byte	0x1
	.byte	0x5e
	.byte	0xc
	.long	0x48
	.quad	.LFB272
	.quad	.LFE272-.LFB272
	.uleb128 0x1
	.byte	0x9c
	.long	0x1b44
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x5e
	.byte	0x1b
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x29
	.string	"out"
	.byte	0x1
	.byte	0x5e
	.byte	0x2a
	.long	0xcae
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x29
	.string	"len"
	.byte	0x1
	.byte	0x5e
	.byte	0x36
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x2b
	.long	.LASF169
	.byte	0x1
	.byte	0x5f
	.byte	0xc
	.long	0x10a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2b
	.long	.LASF27
	.byte	0x1
	.byte	0x60
	.byte	0x12
	.long	0x25e
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x22
	.quad	.LBB2
	.quad	.LBE2-.LBB2
	.uleb128 0x2a
	.string	"i"
	.byte	0x1
	.byte	0x65
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.byte	0
	.uleb128 0x28
	.long	.LASF170
	.byte	0x1
	.byte	0x57
	.byte	0x5
	.long	0x48
	.quad	.LFB271
	.quad	.LFE271-.LFB271
	.uleb128 0x1
	.byte	0x9c
	.long	0x1b94
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x57
	.byte	0x1e
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2e
	.long	.LASF27
	.byte	0x1
	.byte	0x57
	.byte	0x32
	.long	0x25e
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x29
	.string	"len"
	.byte	0x1
	.byte	0x57
	.byte	0x3f
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x28
	.long	.LASF171
	.byte	0x1
	.byte	0x53
	.byte	0x5
	.long	0x48
	.quad	.LFB270
	.quad	.LFE270-.LFB270
	.uleb128 0x1
	.byte	0x9c
	.long	0x1bc6
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x53
	.byte	0x27
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x28
	.long	.LASF172
	.byte	0x1
	.byte	0x4b
	.byte	0x5
	.long	0x48
	.quad	.LFB269
	.quad	.LFE269-.LFB269
	.uleb128 0x1
	.byte	0x9c
	.long	0x1c07
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x4b
	.byte	0x1b
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2e
	.long	.LASF173
	.byte	0x1
	.byte	0x4b
	.byte	0x27
	.long	0x1c07
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xc3
	.uleb128 0x28
	.long	.LASF174
	.byte	0x1
	.byte	0x3b
	.byte	0x5
	.long	0x48
	.quad	.LFB268
	.quad	.LFE268-.LFB268
	.uleb128 0x1
	.byte	0x9c
	.long	0x1c5d
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x3b
	.byte	0x19
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2e
	.long	.LASF173
	.byte	0x1
	.byte	0x3b
	.byte	0x28
	.long	0x3af
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x2e
	.long	.LASF175
	.byte	0x1
	.byte	0x3b
	.byte	0x39
	.long	0x258
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x2f
	.long	.LASF176
	.byte	0x1
	.byte	0x39
	.byte	0x8
	.long	0x30
	.quad	.LFB267
	.quad	.LFE267-.LFB267
	.uleb128 0x1
	.byte	0x9c
	.long	0x1c8f
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x39
	.byte	0x1b
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x2f
	.long	.LASF177
	.byte	0x1
	.byte	0x37
	.byte	0x10
	.long	0x25e
	.quad	.LFB266
	.quad	.LFE266-.LFB266
	.uleb128 0x1
	.byte	0x9c
	.long	0x1cc1
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x37
	.byte	0x24
	.long	0x62c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x28
	.long	.LASF178
	.byte	0x1
	.byte	0x32
	.byte	0x5
	.long	0x48
	.quad	.LFB265
	.quad	.LFE265-.LFB265
	.uleb128 0x1
	.byte	0x9c
	.long	0x1d11
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x32
	.byte	0x13
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.string	"len"
	.byte	0x1
	.byte	0x32
	.byte	0x1f
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x2b
	.long	.LASF179
	.byte	0x1
	.byte	0x33
	.byte	0x12
	.long	0x25e
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x30
	.long	.LASF180
	.byte	0x1
	.byte	0x27
	.byte	0xc
	.long	0x48
	.quad	.LFB264
	.quad	.LFE264-.LFB264
	.uleb128 0x1
	.byte	0x9c
	.long	0x1d5d
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x27
	.byte	0x19
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.string	"p"
	.byte	0x1
	.byte	0x27
	.byte	0x2e
	.long	0x1d5d
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x29
	.string	"n"
	.byte	0x1
	.byte	0x27
	.byte	0x38
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x25e
	.uleb128 0x31
	.long	.LASF188
	.byte	0x1
	.byte	0x22
	.byte	0x6
	.quad	.LFB263
	.quad	.LFE263-.LFB263
	.uleb128 0x1
	.byte	0x9c
	.long	0x1daf
	.uleb128 0x29
	.string	"cbs"
	.byte	0x1
	.byte	0x22
	.byte	0x14
	.long	0x5cc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x2e
	.long	.LASF27
	.byte	0x1
	.byte	0x22
	.byte	0x28
	.long	0x25e
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x29
	.string	"len"
	.byte	0x1
	.byte	0x22
	.byte	0x35
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x20
	.long	.LASF181
	.byte	0x2
	.value	0x3bc
	.byte	0x15
	.long	0xc1
	.quad	.LFB229
	.quad	.LFE229-.LFB229
	.uleb128 0x1
	.byte	0x9c
	.long	0x1e01
	.uleb128 0x1d
	.string	"dst"
	.byte	0x2
	.value	0x3bc
	.byte	0x2a
	.long	0xc1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1d
	.string	"src"
	.byte	0x2
	.value	0x3bc
	.byte	0x3b
	.long	0x11d
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1d
	.string	"n"
	.byte	0x2
	.value	0x3bc
	.byte	0x47
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x20
	.long	.LASF182
	.byte	0x2
	.value	0x3a7
	.byte	0x15
	.long	0xc1
	.quad	.LFB227
	.quad	.LFE227-.LFB227
	.uleb128 0x1
	.byte	0x9c
	.long	0x1e4f
	.uleb128 0x1d
	.string	"s"
	.byte	0x2
	.value	0x3a7
	.byte	0x30
	.long	0x11d
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1d
	.string	"c"
	.byte	0x2
	.value	0x3a7
	.byte	0x37
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.uleb128 0x1d
	.string	"n"
	.byte	0x2
	.value	0x3a7
	.byte	0x41
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x23
	.long	.LASF183
	.byte	0x2
	.value	0x356
	.byte	0x18
	.long	0x10a
	.quad	.LFB225
	.quad	.LFE225-.LFB225
	.uleb128 0x1
	.byte	0x9c
	.long	0x1e81
	.uleb128 0x1d
	.string	"x"
	.byte	0x2
	.value	0x356
	.byte	0x2f
	.long	0x10a
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x23
	.long	.LASF184
	.byte	0x2
	.value	0x352
	.byte	0x18
	.long	0xfe
	.quad	.LFB224
	.quad	.LFE224-.LFB224
	.uleb128 0x1
	.byte	0x9c
	.long	0x1eb3
	.uleb128 0x1d
	.string	"x"
	.byte	0x2
	.value	0x352
	.byte	0x2f
	.long	0xfe
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.uleb128 0x32
	.long	.LASF189
	.byte	0x2
	.value	0x34e
	.byte	0x18
	.long	0xf2
	.quad	.LFB223
	.quad	.LFE223-.LFB223
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x1d
	.string	"x"
	.byte	0x2
	.value	0x34e
	.byte	0x2f
	.long	0xf2
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
	.uleb128 0x26
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x5
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
	.uleb128 0x6
	.uleb128 0xf
	.byte	0
	.uleb128 0xb
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x7
	.uleb128 0xf
	.byte	0
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x8
	.uleb128 0x26
	.byte	0
	.byte	0
	.byte	0
	.uleb128 0x9
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
	.uleb128 0xa
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
	.uleb128 0xb
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
	.uleb128 0xc
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
	.uleb128 0xd
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
	.uleb128 0xe
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
	.uleb128 0xf
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
	.uleb128 0x10
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
	.uleb128 0x11
	.uleb128 0x13
	.byte	0x1
	.uleb128 0x3
	.uleb128 0x8
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
	.uleb128 0x12
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
	.uleb128 0x13
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
	.uleb128 0x14
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
	.uleb128 0x15
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
	.uleb128 0x16
	.uleb128 0x5
	.byte	0
	.uleb128 0x49
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
	.uleb128 0x18
	.byte	0
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
	.uleb128 0x1c
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
	.uleb128 0x1d
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
	.uleb128 0x1e
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
	.uleb128 0x1f
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
	.uleb128 0x20
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
	.uleb128 0x21
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
	.uleb128 0x24
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
	.uleb128 0x25
	.uleb128 0x1
	.byte	0x1
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x26
	.uleb128 0x21
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2f
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x27
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
	.uleb128 0x29
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
	.uleb128 0x2a
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
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x55
	.uleb128 0x17
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
	.uleb128 0x2e
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
	.uleb128 0x2f
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
	.uleb128 0x2117
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
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
	.uleb128 0x2117
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x31
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
	.uleb128 0x2117
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x32
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
	.long	0x3fc
	.value	0x2
	.long	.Ldebug_info0
	.byte	0x8
	.byte	0
	.value	0
	.value	0
	.quad	.LFB223
	.quad	.LFE223-.LFB223
	.quad	.LFB224
	.quad	.LFE224-.LFB224
	.quad	.LFB225
	.quad	.LFE225-.LFB225
	.quad	.LFB227
	.quad	.LFE227-.LFB227
	.quad	.LFB229
	.quad	.LFE229-.LFB229
	.quad	.LFB263
	.quad	.LFE263-.LFB263
	.quad	.LFB264
	.quad	.LFE264-.LFB264
	.quad	.LFB265
	.quad	.LFE265-.LFB265
	.quad	.LFB266
	.quad	.LFE266-.LFB266
	.quad	.LFB267
	.quad	.LFE267-.LFB267
	.quad	.LFB268
	.quad	.LFE268-.LFB268
	.quad	.LFB269
	.quad	.LFE269-.LFB269
	.quad	.LFB270
	.quad	.LFE270-.LFB270
	.quad	.LFB271
	.quad	.LFE271-.LFB271
	.quad	.LFB272
	.quad	.LFE272-.LFB272
	.quad	.LFB273
	.quad	.LFE273-.LFB273
	.quad	.LFB274
	.quad	.LFE274-.LFB274
	.quad	.LFB275
	.quad	.LFE275-.LFB275
	.quad	.LFB276
	.quad	.LFE276-.LFB276
	.quad	.LFB277
	.quad	.LFE277-.LFB277
	.quad	.LFB278
	.quad	.LFE278-.LFB278
	.quad	.LFB279
	.quad	.LFE279-.LFB279
	.quad	.LFB280
	.quad	.LFE280-.LFB280
	.quad	.LFB281
	.quad	.LFE281-.LFB281
	.quad	.LFB282
	.quad	.LFE282-.LFB282
	.quad	.LFB283
	.quad	.LFE283-.LFB283
	.quad	.LFB284
	.quad	.LFE284-.LFB284
	.quad	.LFB285
	.quad	.LFE285-.LFB285
	.quad	.LFB286
	.quad	.LFE286-.LFB286
	.quad	.LFB287
	.quad	.LFE287-.LFB287
	.quad	.LFB288
	.quad	.LFE288-.LFB288
	.quad	.LFB289
	.quad	.LFE289-.LFB289
	.quad	.LFB290
	.quad	.LFE290-.LFB290
	.quad	.LFB291
	.quad	.LFE291-.LFB291
	.quad	.LFB292
	.quad	.LFE292-.LFB292
	.quad	.LFB293
	.quad	.LFE293-.LFB293
	.quad	.LFB294
	.quad	.LFE294-.LFB294
	.quad	.LFB295
	.quad	.LFE295-.LFB295
	.quad	.LFB296
	.quad	.LFE296-.LFB296
	.quad	.LFB297
	.quad	.LFE297-.LFB297
	.quad	.LFB298
	.quad	.LFE298-.LFB298
	.quad	.LFB299
	.quad	.LFE299-.LFB299
	.quad	.LFB300
	.quad	.LFE300-.LFB300
	.quad	.LFB301
	.quad	.LFE301-.LFB301
	.quad	.LFB302
	.quad	.LFE302-.LFB302
	.quad	.LFB303
	.quad	.LFE303-.LFB303
	.quad	.LFB304
	.quad	.LFE304-.LFB304
	.quad	.LFB305
	.quad	.LFE305-.LFB305
	.quad	.LFB306
	.quad	.LFE306-.LFB306
	.quad	.LFB307
	.quad	.LFE307-.LFB307
	.quad	.LFB308
	.quad	.LFE308-.LFB308
	.quad	.LFB309
	.quad	.LFE309-.LFB309
	.quad	.LFB310
	.quad	.LFE310-.LFB310
	.quad	.LFB311
	.quad	.LFE311-.LFB311
	.quad	.LFB312
	.quad	.LFE312-.LFB312
	.quad	.LFB313
	.quad	.LFE313-.LFB313
	.quad	.LFB314
	.quad	.LFE314-.LFB314
	.quad	.LFB315
	.quad	.LFE315-.LFB315
	.quad	.LFB316
	.quad	.LFE316-.LFB316
	.quad	.LFB317
	.quad	.LFE317-.LFB317
	.quad	.LFB318
	.quad	.LFE318-.LFB318
	.quad	.LFB319
	.quad	.LFE319-.LFB319
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LBB3
	.quad	.LBE3
	.quad	.LBB4
	.quad	.LBE4
	.quad	0
	.quad	0
	.quad	.LFB223
	.quad	.LFE223
	.quad	.LFB224
	.quad	.LFE224
	.quad	.LFB225
	.quad	.LFE225
	.quad	.LFB227
	.quad	.LFE227
	.quad	.LFB229
	.quad	.LFE229
	.quad	.LFB263
	.quad	.LFE263
	.quad	.LFB264
	.quad	.LFE264
	.quad	.LFB265
	.quad	.LFE265
	.quad	.LFB266
	.quad	.LFE266
	.quad	.LFB267
	.quad	.LFE267
	.quad	.LFB268
	.quad	.LFE268
	.quad	.LFB269
	.quad	.LFE269
	.quad	.LFB270
	.quad	.LFE270
	.quad	.LFB271
	.quad	.LFE271
	.quad	.LFB272
	.quad	.LFE272
	.quad	.LFB273
	.quad	.LFE273
	.quad	.LFB274
	.quad	.LFE274
	.quad	.LFB275
	.quad	.LFE275
	.quad	.LFB276
	.quad	.LFE276
	.quad	.LFB277
	.quad	.LFE277
	.quad	.LFB278
	.quad	.LFE278
	.quad	.LFB279
	.quad	.LFE279
	.quad	.LFB280
	.quad	.LFE280
	.quad	.LFB281
	.quad	.LFE281
	.quad	.LFB282
	.quad	.LFE282
	.quad	.LFB283
	.quad	.LFE283
	.quad	.LFB284
	.quad	.LFE284
	.quad	.LFB285
	.quad	.LFE285
	.quad	.LFB286
	.quad	.LFE286
	.quad	.LFB287
	.quad	.LFE287
	.quad	.LFB288
	.quad	.LFE288
	.quad	.LFB289
	.quad	.LFE289
	.quad	.LFB290
	.quad	.LFE290
	.quad	.LFB291
	.quad	.LFE291
	.quad	.LFB292
	.quad	.LFE292
	.quad	.LFB293
	.quad	.LFE293
	.quad	.LFB294
	.quad	.LFE294
	.quad	.LFB295
	.quad	.LFE295
	.quad	.LFB296
	.quad	.LFE296
	.quad	.LFB297
	.quad	.LFE297
	.quad	.LFB298
	.quad	.LFE298
	.quad	.LFB299
	.quad	.LFE299
	.quad	.LFB300
	.quad	.LFE300
	.quad	.LFB301
	.quad	.LFE301
	.quad	.LFB302
	.quad	.LFE302
	.quad	.LFB303
	.quad	.LFE303
	.quad	.LFB304
	.quad	.LFE304
	.quad	.LFB305
	.quad	.LFE305
	.quad	.LFB306
	.quad	.LFE306
	.quad	.LFB307
	.quad	.LFE307
	.quad	.LFB308
	.quad	.LFE308
	.quad	.LFB309
	.quad	.LFE309
	.quad	.LFB310
	.quad	.LFE310
	.quad	.LFB311
	.quad	.LFE311
	.quad	.LFB312
	.quad	.LFE312
	.quad	.LFB313
	.quad	.LFE313
	.quad	.LFB314
	.quad	.LFE314
	.quad	.LFB315
	.quad	.LFE315
	.quad	.LFB316
	.quad	.LFE316
	.quad	.LFB317
	.quad	.LFE317
	.quad	.LFB318
	.quad	.LFE318
	.quad	.LFB319
	.quad	.LFE319
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF73:
	.string	"allow_timezone_offset"
.LASF30:
	.string	"tm_hour"
.LASF102:
	.string	"bit_num"
.LASF54:
	.string	"aws_lc_0_38_0_CBB_init"
.LASF8:
	.string	"size_t"
.LASF143:
	.string	"len64"
.LASF43:
	.string	"cbb_child_st"
.LASF20:
	.string	"uint64_t"
.LASF124:
	.string	"cbs_get_asn1"
.LASF9:
	.string	"__uint8_t"
.LASF109:
	.string	"aws_lc_0_38_0_CBS_get_optional_asn1_uint64"
.LASF92:
	.string	"prev"
.LASF127:
	.string	"throwaway"
.LASF64:
	.string	"aws_lc_0_38_0_OPENSSL_strndup"
.LASF147:
	.string	"parse_base128_integer"
.LASF46:
	.string	"pending_len_len"
.LASF107:
	.string	"child2"
.LASF22:
	.string	"CBS_ASN1_TAG"
.LASF51:
	.string	"aws_lc_0_38_0_OPENSSL_gmtime_adj"
.LASF24:
	.string	"is_child"
.LASF186:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/cbs.c"
.LASF13:
	.string	"__int64_t"
.LASF89:
	.string	"aws_lc_0_38_0_CBS_asn1_oid_to_text"
.LASF181:
	.string	"OPENSSL_memcpy"
.LASF59:
	.string	"aws_lc_0_38_0_OPENSSL_isdigit"
.LASF161:
	.string	"aws_lc_0_38_0_CBS_get_u64"
.LASF165:
	.string	"aws_lc_0_38_0_CBS_get_u16le"
.LASF2:
	.string	"long long int"
.LASF7:
	.string	"signed char"
.LASF110:
	.string	"aws_lc_0_38_0_CBS_get_optional_asn1_octet_string"
.LASF63:
	.string	"memchr"
.LASF128:
	.string	"__PRETTY_FUNCTION__"
.LASF119:
	.string	"aws_lc_0_38_0_CBS_peek_asn1_tag"
.LASF149:
	.string	"seen_digit"
.LASF117:
	.string	"sign_extend"
.LASF41:
	.string	"can_resize"
.LASF135:
	.string	"aws_lc_0_38_0_CBS_get_any_asn1_element"
.LASF157:
	.string	"aws_lc_0_38_0_CBS_copy_bytes"
.LASF158:
	.string	"aws_lc_0_38_0_CBS_get_bytes"
.LASF76:
	.string	"year"
.LASF90:
	.string	"txt_len"
.LASF69:
	.string	"present"
.LASF0:
	.string	"long int"
.LASF173:
	.string	"out_ptr"
.LASF101:
	.string	"byte_num"
.LASF138:
	.string	"ber_ok"
.LASF177:
	.string	"aws_lc_0_38_0_CBS_data"
.LASF18:
	.string	"uint16_t"
.LASF39:
	.string	"double"
.LASF65:
	.string	"aws_lc_0_38_0_OPENSSL_memdup"
.LASF38:
	.string	"__tm_zone"
.LASF114:
	.string	"bytes"
.LASF153:
	.string	"aws_lc_0_38_0_CBS_get_u16_length_prefixed"
.LASF12:
	.string	"__uint32_t"
.LASF100:
	.string	"aws_lc_0_38_0_CBS_asn1_bitstring_has_bit"
.LASF32:
	.string	"tm_mon"
.LASF97:
	.string	"out_is_negative"
.LASF179:
	.string	"dummy"
.LASF21:
	.string	"long long unsigned int"
.LASF115:
	.string	"value"
.LASF81:
	.string	"offset_seconds"
.LASF33:
	.string	"tm_year"
.LASF77:
	.string	"month"
.LASF6:
	.string	"unsigned int"
.LASF48:
	.string	"__int128"
.LASF178:
	.string	"aws_lc_0_38_0_CBS_skip"
.LASF151:
	.string	"split"
.LASF1:
	.string	"long unsigned int"
.LASF94:
	.string	"aws_lc_0_38_0_CBS_is_unsigned_asn1_integer"
.LASF171:
	.string	"aws_lc_0_38_0_CBS_contains_zero_byte"
.LASF133:
	.string	"out_indefinite"
.LASF37:
	.string	"__tm_gmtoff"
.LASF27:
	.string	"data"
.LASF5:
	.string	"short unsigned int"
.LASF176:
	.string	"aws_lc_0_38_0_CBS_len"
.LASF36:
	.string	"tm_isdst"
.LASF85:
	.string	"is_valid_day"
.LASF56:
	.string	"strlen"
.LASF105:
	.string	"last"
.LASF67:
	.string	"aws_lc_0_38_0_OPENSSL_free"
.LASF86:
	.string	"cbs_get_two_digits"
.LASF88:
	.string	"second_digit"
.LASF103:
	.string	"aws_lc_0_38_0_CBS_is_valid_asn1_bitstring"
.LASF116:
	.string	"aws_lc_0_38_0_CBS_get_asn1_int64"
.LASF166:
	.string	"aws_lc_0_38_0_CBS_get_u16"
.LASF79:
	.string	"copy"
.LASF141:
	.string	"length_byte"
.LASF129:
	.string	"aws_lc_0_38_0_CBS_get_any_ber_asn1_element"
.LASF168:
	.string	"cbs_get_u"
.LASF93:
	.string	"add_decimal"
.LASF29:
	.string	"tm_min"
.LASF154:
	.string	"aws_lc_0_38_0_CBS_get_u8_length_prefixed"
.LASF155:
	.string	"cbs_get_length_prefixed"
.LASF170:
	.string	"aws_lc_0_38_0_CBS_mem_equal"
.LASF35:
	.string	"tm_yday"
.LASF174:
	.string	"aws_lc_0_38_0_CBS_stow"
.LASF25:
	.string	"cbb_st"
.LASF82:
	.string	"offset_hours"
.LASF121:
	.string	"actual_tag"
.LASF120:
	.string	"tag_value"
.LASF91:
	.string	"aws_lc_0_38_0_CBS_is_valid_asn1_oid"
.LASF113:
	.string	"aws_lc_0_38_0_CBS_get_asn1_bool"
.LASF78:
	.string	"hour"
.LASF14:
	.string	"__uint64_t"
.LASF80:
	.string	"offset_sign"
.LASF47:
	.string	"pending_is_asn1"
.LASF42:
	.string	"error"
.LASF164:
	.string	"aws_lc_0_38_0_CBS_get_u24"
.LASF104:
	.string	"num_unused_bits"
.LASF66:
	.string	"aws_lc_0_38_0_CBB_cleanup"
.LASF60:
	.string	"__assert_fail"
.LASF137:
	.string	"aws_lc_0_38_0_cbs_get_any_asn1_element"
.LASF180:
	.string	"cbs_get"
.LASF136:
	.string	"aws_lc_0_38_0_CBS_get_any_asn1"
.LASF49:
	.string	"__int128 unsigned"
.LASF44:
	.string	"base"
.LASF50:
	.string	"_Bool"
.LASF162:
	.string	"aws_lc_0_38_0_CBS_get_u32le"
.LASF187:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF23:
	.string	"child"
.LASF167:
	.string	"aws_lc_0_38_0_CBS_get_u8"
.LASF26:
	.string	"cbs_st"
.LASF10:
	.string	"short int"
.LASF106:
	.string	"aws_lc_0_38_0_CBS_get_optional_asn1_bool"
.LASF87:
	.string	"first_digit"
.LASF139:
	.string	"universal_tag_ok"
.LASF83:
	.string	"offset_minutes"
.LASF185:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF125:
	.string	"skip_header"
.LASF98:
	.string	"first_byte"
.LASF118:
	.string	"aws_lc_0_38_0_CBS_get_asn1_uint64"
.LASF111:
	.string	"out_present"
.LASF34:
	.string	"tm_wday"
.LASF72:
	.string	"out_tm"
.LASF3:
	.string	"long double"
.LASF15:
	.string	"char"
.LASF99:
	.string	"second_byte"
.LASF11:
	.string	"__uint16_t"
.LASF108:
	.string	"boolean"
.LASF96:
	.string	"aws_lc_0_38_0_CBS_is_valid_asn1_integer"
.LASF182:
	.string	"OPENSSL_memchr"
.LASF131:
	.string	"out_header_len"
.LASF152:
	.string	"aws_lc_0_38_0_CBS_get_u24_length_prefixed"
.LASF183:
	.string	"CRYPTO_bswap8"
.LASF45:
	.string	"offset"
.LASF61:
	.string	"memcpy"
.LASF95:
	.string	"is_negative"
.LASF4:
	.string	"unsigned char"
.LASF145:
	.string	"tag_byte"
.LASF31:
	.string	"tm_mday"
.LASF122:
	.string	"aws_lc_0_38_0_CBS_get_asn1_element"
.LASF71:
	.string	"aws_lc_0_38_0_CBS_parse_utc_time"
.LASF74:
	.string	"aws_lc_0_38_0_CBS_parse_generalized_time"
.LASF142:
	.string	"num_bytes"
.LASF57:
	.string	"snprintf"
.LASF159:
	.string	"aws_lc_0_38_0_CBS_get_last_u8"
.LASF19:
	.string	"uint32_t"
.LASF28:
	.string	"tm_sec"
.LASF156:
	.string	"len_len"
.LASF112:
	.string	"aws_lc_0_38_0_CBS_get_optional_asn1"
.LASF58:
	.string	"memset"
.LASF163:
	.string	"aws_lc_0_38_0_CBS_get_u32"
.LASF172:
	.string	"aws_lc_0_38_0_CBS_strdup"
.LASF144:
	.string	"parse_asn1_tag"
.LASF189:
	.string	"CRYPTO_bswap2"
.LASF146:
	.string	"tag_number"
.LASF17:
	.string	"uint8_t"
.LASF68:
	.string	"default_value"
.LASF188:
	.string	"aws_lc_0_38_0_CBS_init"
.LASF55:
	.string	"aws_lc_0_38_0_CBB_add_bytes"
.LASF134:
	.string	"ber_found_temp"
.LASF123:
	.string	"aws_lc_0_38_0_CBS_get_asn1"
.LASF150:
	.string	"aws_lc_0_38_0_CBS_get_until_first"
.LASF84:
	.string	"CBS_parse_rfc5280_time_internal"
.LASF148:
	.string	"aws_lc_0_38_0_CBS_get_u64_decimal"
.LASF70:
	.string	"aws_lc_0_38_0_CBS_get_optional_asn1_int64"
.LASF184:
	.string	"CRYPTO_bswap4"
.LASF175:
	.string	"out_len"
.LASF16:
	.string	"int64_t"
.LASF160:
	.string	"aws_lc_0_38_0_CBS_get_u64le"
.LASF40:
	.string	"cbb_buffer_st"
.LASF130:
	.string	"out_tag"
.LASF52:
	.string	"aws_lc_0_38_0_CBB_finish"
.LASF75:
	.string	"is_gentime"
.LASF126:
	.string	"header_len"
.LASF169:
	.string	"result"
.LASF140:
	.string	"header"
.LASF53:
	.string	"aws_lc_0_38_0_CBB_add_u8"
.LASF62:
	.string	"aws_lc_0_38_0_CRYPTO_memcmp"
.LASF132:
	.string	"out_ber_found"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
