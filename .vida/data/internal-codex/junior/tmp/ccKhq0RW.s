	.file	"buf.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/buf/buf.c"
	.section	.text.OPENSSL_memcpy,"ax",@progbits
	.type	OPENSSL_memcpy, @function
OPENSSL_memcpy:
.LFB94:
	.file 2 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/buf/../internal.h"
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
	jne	.L2
	.loc 2 958 12
	movq	-8(%rbp), %rax
	jmp	.L3
.L2:
	.loc 2 961 10
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	memcpy@PLT
.L3:
	.loc 2 962 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE94:
	.size	OPENSSL_memcpy, .-OPENSSL_memcpy
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
	jne	.L5
	.loc 2 974 12
	movq	-8(%rbp), %rax
	jmp	.L6
.L5:
	.loc 2 977 10
	movq	-24(%rbp), %rdx
	movl	-12(%rbp), %ecx
	movq	-8(%rbp), %rax
	movl	%ecx, %esi
	movq	%rax, %rdi
	call	memset@PLT
.L6:
	.loc 2 978 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE96:
	.size	OPENSSL_memset, .-OPENSSL_memset
	.section	.text.aws_lc_0_38_0_BUF_MEM_new,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_MEM_new
	.type	aws_lc_0_38_0_BUF_MEM_new, @function
aws_lc_0_38_0_BUF_MEM_new:
.LFB128:
	.loc 1 67 28
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 1 67 37
	movl	$24, %edi
	call	aws_lc_0_38_0_OPENSSL_zalloc@PLT
	.loc 1 67 70
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE128:
	.size	aws_lc_0_38_0_BUF_MEM_new, .-aws_lc_0_38_0_BUF_MEM_new
	.section	.text.aws_lc_0_38_0_BUF_MEM_free,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_MEM_free
	.type	aws_lc_0_38_0_BUF_MEM_free, @function
aws_lc_0_38_0_BUF_MEM_free:
.LFB129:
	.loc 1 69 33
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	.loc 1 70 6
	cmpq	$0, -8(%rbp)
	je	.L12
	.loc 1 74 19
	movq	-8(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 74 3
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_free@PLT
	.loc 1 75 3
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_free@PLT
	jmp	.L9
.L12:
	.loc 1 71 5
	nop
.L9:
	.loc 1 76 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE129:
	.size	aws_lc_0_38_0_BUF_MEM_free, .-aws_lc_0_38_0_BUF_MEM_free
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/buf/buf.c"
	.section	.text.aws_lc_0_38_0_BUF_MEM_reserve,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_MEM_reserve
	.type	aws_lc_0_38_0_BUF_MEM_reserve, @function
aws_lc_0_38_0_BUF_MEM_reserve:
.LFB130:
	.loc 1 78 47
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	.loc 1 79 10
	movq	-40(%rbp), %rax
	movq	16(%rax), %rax
	.loc 1 79 6
	cmpq	-48(%rbp), %rax
	jb	.L14
	.loc 1 80 12
	movl	$1, %eax
	jmp	.L15
.L14:
	.loc 1 83 10
	movq	-48(%rbp), %rax
	addq	$3, %rax
	movq	%rax, -8(%rbp)
	.loc 1 84 6
	movq	-8(%rbp), %rax
	cmpq	-48(%rbp), %rax
	jnb	.L16
	.loc 1 85 5
	movl	$85, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$69, %edx
	movl	$0, %esi
	movl	$7, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 86 12
	movl	$0, %eax
	jmp	.L15
.L16:
	.loc 1 88 5
	movq	-8(%rbp), %rax
	movabsq	$-6148914691236517205, %rdx
	mulq	%rdx
	movq	%rdx, %rax
	shrq	%rax
	movq	%rax, -8(%rbp)
	.loc 1 89 10
	movq	-8(%rbp), %rax
	salq	$2, %rax
	movq	%rax, -16(%rbp)
	.loc 1 90 18
	movq	-16(%rbp), %rax
	shrq	$2, %rax
	.loc 1 90 6
	cmpq	%rax, -8(%rbp)
	je	.L17
	.loc 1 91 5
	movl	$91, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$69, %edx
	movl	$0, %esi
	movl	$7, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 92 12
	movl	$0, %eax
	jmp	.L15
.L17:
	.loc 1 95 38
	movq	-40(%rbp), %rax
	movq	8(%rax), %rax
	.loc 1 95 19
	movq	-16(%rbp), %rdx
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_realloc@PLT
	movq	%rax, -24(%rbp)
	.loc 1 96 6
	cmpq	$0, -24(%rbp)
	jne	.L18
	.loc 1 97 12
	movl	$0, %eax
	jmp	.L15
.L18:
	.loc 1 100 13
	movq	-40(%rbp), %rax
	movq	-24(%rbp), %rdx
	movq	%rdx, 8(%rax)
	.loc 1 101 12
	movq	-40(%rbp), %rax
	movq	-16(%rbp), %rdx
	movq	%rdx, 16(%rax)
	.loc 1 102 10
	movl	$1, %eax
.L15:
	.loc 1 103 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE130:
	.size	aws_lc_0_38_0_BUF_MEM_reserve, .-aws_lc_0_38_0_BUF_MEM_reserve
	.section	.text.aws_lc_0_38_0_BUF_MEM_grow,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_MEM_grow
	.type	aws_lc_0_38_0_BUF_MEM_grow, @function
aws_lc_0_38_0_BUF_MEM_grow:
.LFB131:
	.loc 1 105 47
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 106 8
	movq	-16(%rbp), %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_BUF_MEM_reserve@PLT
	.loc 1 106 6 discriminator 1
	testl	%eax, %eax
	jne	.L20
	.loc 1 107 12
	movl	$0, %eax
	jmp	.L21
.L20:
	.loc 1 109 10
	movq	-8(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 109 6
	cmpq	-16(%rbp), %rax
	jnb	.L22
	.loc 1 110 57
	movq	-8(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 110 5
	movq	-16(%rbp), %rdx
	subq	%rax, %rdx
	.loc 1 110 24
	movq	-8(%rbp), %rax
	movq	8(%rax), %rcx
	.loc 1 110 34
	movq	-8(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 110 20
	addq	%rcx, %rax
	.loc 1 110 5
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
.L22:
	.loc 1 112 15
	movq	-8(%rbp), %rax
	movq	-16(%rbp), %rdx
	movq	%rdx, (%rax)
	.loc 1 113 10
	movq	-16(%rbp), %rax
.L21:
	.loc 1 114 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE131:
	.size	aws_lc_0_38_0_BUF_MEM_grow, .-aws_lc_0_38_0_BUF_MEM_grow
	.section	.text.aws_lc_0_38_0_BUF_MEM_grow_clean,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_MEM_grow_clean
	.type	aws_lc_0_38_0_BUF_MEM_grow_clean, @function
aws_lc_0_38_0_BUF_MEM_grow_clean:
.LFB132:
	.loc 1 116 53
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 117 10
	movq	-16(%rbp), %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_BUF_MEM_grow@PLT
	.loc 1 118 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE132:
	.size	aws_lc_0_38_0_BUF_MEM_grow_clean, .-aws_lc_0_38_0_BUF_MEM_grow_clean
	.section	.text.aws_lc_0_38_0_BUF_MEM_append,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_MEM_append
	.type	aws_lc_0_38_0_BUF_MEM_append, @function
aws_lc_0_38_0_BUF_MEM_append:
.LFB133:
	.loc 1 120 62
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
	.loc 1 122 6
	cmpq	$0, -40(%rbp)
	jne	.L26
	.loc 1 123 12
	movl	$1, %eax
	jmp	.L27
.L26:
	.loc 1 125 23
	movq	-24(%rbp), %rax
	movq	(%rax), %rdx
	.loc 1 125 10
	movq	-40(%rbp), %rax
	addq	%rdx, %rax
	movq	%rax, -8(%rbp)
	.loc 1 126 6
	movq	-8(%rbp), %rax
	cmpq	-40(%rbp), %rax
	jnb	.L28
	.loc 1 127 5
	movl	$127, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$69, %edx
	movl	$0, %esi
	movl	$7, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 128 12
	movl	$0, %eax
	jmp	.L27
.L28:
	.loc 1 130 8
	movq	-8(%rbp), %rdx
	movq	-24(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_BUF_MEM_reserve@PLT
	.loc 1 130 6 discriminator 1
	testl	%eax, %eax
	jne	.L29
	.loc 1 131 12
	movl	$0, %eax
	jmp	.L27
.L29:
	.loc 1 133 21
	movq	-24(%rbp), %rax
	movq	8(%rax), %rdx
	.loc 1 133 33
	movq	-24(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 133 28
	leaq	(%rdx,%rax), %rcx
	.loc 1 133 3
	movq	-40(%rbp), %rdx
	movq	-32(%rbp), %rax
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
	.loc 1 134 15
	movq	-24(%rbp), %rax
	movq	-8(%rbp), %rdx
	movq	%rdx, (%rax)
	.loc 1 135 10
	movl	$1, %eax
.L27:
	.loc 1 136 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE133:
	.size	aws_lc_0_38_0_BUF_MEM_append, .-aws_lc_0_38_0_BUF_MEM_append
	.section	.text.aws_lc_0_38_0_BUF_strdup,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_strdup
	.type	aws_lc_0_38_0_BUF_strdup, @function
aws_lc_0_38_0_BUF_strdup:
.LFB134:
	.loc 1 138 35
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	.loc 1 138 44
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_strdup@PLT
	.loc 1 138 65
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE134:
	.size	aws_lc_0_38_0_BUF_strdup, .-aws_lc_0_38_0_BUF_strdup
	.section	.text.aws_lc_0_38_0_BUF_strnlen,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_strnlen
	.type	aws_lc_0_38_0_BUF_strnlen, @function
aws_lc_0_38_0_BUF_strnlen:
.LFB135:
	.loc 1 140 53
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 141 10
	movq	-16(%rbp), %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_strnlen@PLT
	.loc 1 142 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE135:
	.size	aws_lc_0_38_0_BUF_strnlen, .-aws_lc_0_38_0_BUF_strnlen
	.section	.text.aws_lc_0_38_0_BUF_strndup,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_strndup
	.type	aws_lc_0_38_0_BUF_strndup, @function
aws_lc_0_38_0_BUF_strndup:
.LFB136:
	.loc 1 144 49
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 145 10
	movq	-16(%rbp), %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_strndup@PLT
	.loc 1 146 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE136:
	.size	aws_lc_0_38_0_BUF_strndup, .-aws_lc_0_38_0_BUF_strndup
	.section	.text.aws_lc_0_38_0_BUF_strlcpy,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_strlcpy
	.type	aws_lc_0_38_0_BUF_strlcpy, @function
aws_lc_0_38_0_BUF_strlcpy:
.LFB137:
	.loc 1 148 65
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
	.loc 1 149 10
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_strlcpy@PLT
	.loc 1 150 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE137:
	.size	aws_lc_0_38_0_BUF_strlcpy, .-aws_lc_0_38_0_BUF_strlcpy
	.section	.text.aws_lc_0_38_0_BUF_strlcat,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_strlcat
	.type	aws_lc_0_38_0_BUF_strlcat, @function
aws_lc_0_38_0_BUF_strlcat:
.LFB138:
	.loc 1 152 65
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
	.loc 1 153 10
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_strlcat@PLT
	.loc 1 154 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE138:
	.size	aws_lc_0_38_0_BUF_strlcat, .-aws_lc_0_38_0_BUF_strlcat
	.section	.text.aws_lc_0_38_0_BUF_memdup,"ax",@progbits
	.globl	aws_lc_0_38_0_BUF_memdup
	.type	aws_lc_0_38_0_BUF_memdup, @function
aws_lc_0_38_0_BUF_memdup:
.LFB139:
	.loc 1 156 49
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 157 10
	movq	-16(%rbp), %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_memdup@PLT
	.loc 1 158 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE139:
	.size	aws_lc_0_38_0_BUF_memdup, .-aws_lc_0_38_0_BUF_memdup
	.text
.Letext0:
	.file 3 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 4 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.file 5 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/buf.h"
	.file 6 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/mem.h"
	.file 7 "/usr/include/string.h"
	.file 8 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/err.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x620
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF47
	.byte	0xc
	.long	.LASF48
	.long	.LASF49
	.long	.Ldebug_ranges0+0
	.quad	0
	.long	.Ldebug_line0
	.uleb128 0x2
	.byte	0x8
	.byte	0x5
	.long	.LASF0
	.uleb128 0x3
	.long	.LASF11
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
	.uleb128 0x2
	.byte	0x2
	.byte	0x5
	.long	.LASF8
	.uleb128 0x5
	.byte	0x8
	.uleb128 0x6
	.byte	0x8
	.long	0x83
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF9
	.uleb128 0x7
	.long	0x83
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF10
	.uleb128 0x6
	.byte	0x8
	.long	0x9c
	.uleb128 0x8
	.uleb128 0x9
	.long	.LASF12
	.byte	0x4
	.value	0x192
	.byte	0x1b
	.long	0xaa
	.uleb128 0xa
	.long	.LASF50
	.byte	0x18
	.byte	0x5
	.byte	0x47
	.byte	0x8
	.long	0xdf
	.uleb128 0xb
	.long	.LASF13
	.byte	0x5
	.byte	0x48
	.byte	0xa
	.long	0x30
	.byte	0
	.uleb128 0xb
	.long	.LASF14
	.byte	0x5
	.byte	0x49
	.byte	0x9
	.long	0x7d
	.byte	0x8
	.uleb128 0xc
	.string	"max"
	.byte	0x5
	.byte	0x4a
	.byte	0xa
	.long	0x30
	.byte	0x10
	.byte	0
	.uleb128 0x6
	.byte	0x8
	.long	0x8a
	.uleb128 0x2
	.byte	0x8
	.byte	0x4
	.long	.LASF15
	.uleb128 0x2
	.byte	0x10
	.byte	0x5
	.long	.LASF16
	.uleb128 0x2
	.byte	0x10
	.byte	0x7
	.long	.LASF17
	.uleb128 0x2
	.byte	0x1
	.byte	0x2
	.long	.LASF18
	.uleb128 0xd
	.long	.LASF19
	.byte	0x6
	.byte	0xce
	.byte	0x16
	.long	0x7b
	.long	0x11c
	.uleb128 0xe
	.long	0x96
	.uleb128 0xe
	.long	0x30
	.byte	0
	.uleb128 0xd
	.long	.LASF20
	.byte	0x6
	.byte	0xd5
	.byte	0x17
	.long	0x30
	.long	0x13c
	.uleb128 0xe
	.long	0x7d
	.uleb128 0xe
	.long	0xdf
	.uleb128 0xe
	.long	0x30
	.byte	0
	.uleb128 0xd
	.long	.LASF21
	.byte	0x6
	.byte	0xd1
	.byte	0x17
	.long	0x30
	.long	0x15c
	.uleb128 0xe
	.long	0x7d
	.uleb128 0xe
	.long	0xdf
	.uleb128 0xe
	.long	0x30
	.byte	0
	.uleb128 0xd
	.long	.LASF22
	.byte	0x6
	.byte	0xc9
	.byte	0x16
	.long	0x7d
	.long	0x177
	.uleb128 0xe
	.long	0xdf
	.uleb128 0xe
	.long	0x30
	.byte	0
	.uleb128 0xd
	.long	.LASF23
	.byte	0x6
	.byte	0x80
	.byte	0x17
	.long	0x30
	.long	0x192
	.uleb128 0xe
	.long	0xdf
	.uleb128 0xe
	.long	0x30
	.byte	0
	.uleb128 0xd
	.long	.LASF24
	.byte	0x6
	.byte	0x7d
	.byte	0x16
	.long	0x7d
	.long	0x1a8
	.uleb128 0xe
	.long	0xdf
	.byte	0
	.uleb128 0xd
	.long	.LASF25
	.byte	0x7
	.byte	0x2b
	.byte	0xe
	.long	0x7b
	.long	0x1c8
	.uleb128 0xe
	.long	0x7b
	.uleb128 0xe
	.long	0x96
	.uleb128 0xe
	.long	0x3c
	.byte	0
	.uleb128 0xd
	.long	.LASF26
	.byte	0x7
	.byte	0x3d
	.byte	0xe
	.long	0x7b
	.long	0x1e8
	.uleb128 0xe
	.long	0x7b
	.uleb128 0xe
	.long	0x43
	.uleb128 0xe
	.long	0x3c
	.byte	0
	.uleb128 0xd
	.long	.LASF27
	.byte	0x6
	.byte	0x63
	.byte	0x16
	.long	0x7b
	.long	0x203
	.uleb128 0xe
	.long	0x7b
	.uleb128 0xe
	.long	0x30
	.byte	0
	.uleb128 0xf
	.long	.LASF28
	.byte	0x8
	.value	0x1d0
	.byte	0x15
	.long	0x22a
	.uleb128 0xe
	.long	0x43
	.uleb128 0xe
	.long	0x43
	.uleb128 0xe
	.long	0x43
	.uleb128 0xe
	.long	0xdf
	.uleb128 0xe
	.long	0x66
	.byte	0
	.uleb128 0x10
	.long	.LASF29
	.byte	0x6
	.byte	0x69
	.byte	0x15
	.long	0x23c
	.uleb128 0xe
	.long	0x7b
	.byte	0
	.uleb128 0xd
	.long	.LASF30
	.byte	0x6
	.byte	0x57
	.byte	0x16
	.long	0x7b
	.long	0x252
	.uleb128 0xe
	.long	0x30
	.byte	0
	.uleb128 0x11
	.long	.LASF32
	.byte	0x1
	.byte	0x9c
	.byte	0x7
	.long	0x7b
	.quad	.LFB139
	.quad	.LFE139-.LFB139
	.uleb128 0x1
	.byte	0x9c
	.long	0x293
	.uleb128 0x12
	.long	.LASF14
	.byte	0x1
	.byte	0x9c
	.byte	0x1e
	.long	0x96
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x12
	.long	.LASF31
	.byte	0x1
	.byte	0x9c
	.byte	0x2b
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x11
	.long	.LASF33
	.byte	0x1
	.byte	0x98
	.byte	0x8
	.long	0x30
	.quad	.LFB138
	.quad	.LFE138-.LFB138
	.uleb128 0x1
	.byte	0x9c
	.long	0x2e3
	.uleb128 0x13
	.string	"dst"
	.byte	0x1
	.byte	0x98
	.byte	0x1a
	.long	0x7d
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x13
	.string	"src"
	.byte	0x1
	.byte	0x98
	.byte	0x2b
	.long	0xdf
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x12
	.long	.LASF34
	.byte	0x1
	.byte	0x98
	.byte	0x37
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x11
	.long	.LASF35
	.byte	0x1
	.byte	0x94
	.byte	0x8
	.long	0x30
	.quad	.LFB137
	.quad	.LFE137-.LFB137
	.uleb128 0x1
	.byte	0x9c
	.long	0x333
	.uleb128 0x13
	.string	"dst"
	.byte	0x1
	.byte	0x94
	.byte	0x1a
	.long	0x7d
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x13
	.string	"src"
	.byte	0x1
	.byte	0x94
	.byte	0x2b
	.long	0xdf
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x12
	.long	.LASF34
	.byte	0x1
	.byte	0x94
	.byte	0x37
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x11
	.long	.LASF36
	.byte	0x1
	.byte	0x90
	.byte	0x7
	.long	0x7d
	.quad	.LFB136
	.quad	.LFE136-.LFB136
	.uleb128 0x1
	.byte	0x9c
	.long	0x374
	.uleb128 0x13
	.string	"str"
	.byte	0x1
	.byte	0x90
	.byte	0x1f
	.long	0xdf
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x12
	.long	.LASF31
	.byte	0x1
	.byte	0x90
	.byte	0x2b
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x11
	.long	.LASF37
	.byte	0x1
	.byte	0x8c
	.byte	0x8
	.long	0x30
	.quad	.LFB135
	.quad	.LFE135-.LFB135
	.uleb128 0x1
	.byte	0x9c
	.long	0x3b5
	.uleb128 0x13
	.string	"str"
	.byte	0x1
	.byte	0x8c
	.byte	0x20
	.long	0xdf
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x12
	.long	.LASF38
	.byte	0x1
	.byte	0x8c
	.byte	0x2c
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x11
	.long	.LASF39
	.byte	0x1
	.byte	0x8a
	.byte	0x7
	.long	0x7d
	.quad	.LFB134
	.quad	.LFE134-.LFB134
	.uleb128 0x1
	.byte	0x9c
	.long	0x3e7
	.uleb128 0x13
	.string	"str"
	.byte	0x1
	.byte	0x8a
	.byte	0x1e
	.long	0xdf
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x11
	.long	.LASF40
	.byte	0x1
	.byte	0x78
	.byte	0x5
	.long	0x43
	.quad	.LFB133
	.quad	.LFE133-.LFB133
	.uleb128 0x1
	.byte	0x9c
	.long	0x445
	.uleb128 0x13
	.string	"buf"
	.byte	0x1
	.byte	0x78
	.byte	0x1d
	.long	0x445
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x13
	.string	"in"
	.byte	0x1
	.byte	0x78
	.byte	0x2e
	.long	0x96
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x13
	.string	"len"
	.byte	0x1
	.byte	0x78
	.byte	0x39
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x14
	.long	.LASF44
	.byte	0x1
	.byte	0x7d
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x6
	.byte	0x8
	.long	0x9d
	.uleb128 0x11
	.long	.LASF41
	.byte	0x1
	.byte	0x74
	.byte	0x8
	.long	0x30
	.quad	.LFB132
	.quad	.LFE132-.LFB132
	.uleb128 0x1
	.byte	0x9c
	.long	0x48c
	.uleb128 0x13
	.string	"buf"
	.byte	0x1
	.byte	0x74
	.byte	0x24
	.long	0x445
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x13
	.string	"len"
	.byte	0x1
	.byte	0x74
	.byte	0x30
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x11
	.long	.LASF42
	.byte	0x1
	.byte	0x69
	.byte	0x8
	.long	0x30
	.quad	.LFB131
	.quad	.LFE131-.LFB131
	.uleb128 0x1
	.byte	0x9c
	.long	0x4cd
	.uleb128 0x13
	.string	"buf"
	.byte	0x1
	.byte	0x69
	.byte	0x1e
	.long	0x445
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x13
	.string	"len"
	.byte	0x1
	.byte	0x69
	.byte	0x2a
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x11
	.long	.LASF43
	.byte	0x1
	.byte	0x4e
	.byte	0x5
	.long	0x43
	.quad	.LFB130
	.quad	.LFE130-.LFB130
	.uleb128 0x1
	.byte	0x9c
	.long	0x539
	.uleb128 0x13
	.string	"buf"
	.byte	0x1
	.byte	0x4e
	.byte	0x1e
	.long	0x445
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x13
	.string	"cap"
	.byte	0x1
	.byte	0x4e
	.byte	0x2a
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x15
	.string	"n"
	.byte	0x1
	.byte	0x53
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x14
	.long	.LASF45
	.byte	0x1
	.byte	0x59
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x14
	.long	.LASF46
	.byte	0x1
	.byte	0x5f
	.byte	0x9
	.long	0x7d
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x16
	.long	.LASF51
	.byte	0x1
	.byte	0x45
	.byte	0x6
	.quad	.LFB129
	.quad	.LFE129-.LFB129
	.uleb128 0x1
	.byte	0x9c
	.long	0x567
	.uleb128 0x13
	.string	"buf"
	.byte	0x1
	.byte	0x45
	.byte	0x1c
	.long	0x445
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x17
	.long	.LASF52
	.byte	0x1
	.byte	0x43
	.byte	0xa
	.long	0x445
	.quad	.LFB128
	.quad	.LFE128-.LFB128
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x18
	.long	.LASF53
	.byte	0x2
	.value	0x3cc
	.byte	0x15
	.long	0x7b
	.quad	.LFB96
	.quad	.LFE96-.LFB96
	.uleb128 0x1
	.byte	0x9c
	.long	0x5d5
	.uleb128 0x19
	.string	"dst"
	.byte	0x2
	.value	0x3cc
	.byte	0x2a
	.long	0x7b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x19
	.string	"c"
	.byte	0x2
	.value	0x3cc
	.byte	0x33
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.uleb128 0x19
	.string	"n"
	.byte	0x2
	.value	0x3cc
	.byte	0x3d
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x1a
	.long	.LASF54
	.byte	0x2
	.value	0x3bc
	.byte	0x15
	.long	0x7b
	.quad	.LFB94
	.quad	.LFE94-.LFB94
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x19
	.string	"dst"
	.byte	0x2
	.value	0x3bc
	.byte	0x2a
	.long	0x7b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x19
	.string	"src"
	.byte	0x2
	.value	0x3bc
	.byte	0x3b
	.long	0x96
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x19
	.string	"n"
	.byte	0x2
	.value	0x3bc
	.byte	0x47
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
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
	.uleb128 0xf
	.byte	0
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x7
	.uleb128 0x26
	.byte	0
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
	.uleb128 0xb
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
	.uleb128 0xc
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
	.uleb128 0xd
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
	.uleb128 0xe
	.uleb128 0x5
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0xf
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
	.uleb128 0x10
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
	.uleb128 0x11
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
	.uleb128 0x12
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
	.uleb128 0x13
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
	.uleb128 0x14
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
	.uleb128 0x15
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
	.uleb128 0x17
	.uleb128 0x2e
	.byte	0
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
	.byte	0
	.byte	0
	.uleb128 0x18
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
	.uleb128 0x19
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
	.uleb128 0x1a
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
	.byte	0
	.byte	0
	.byte	0
	.section	.debug_aranges,"",@progbits
	.long	0xfc
	.value	0x2
	.long	.Ldebug_info0
	.byte	0x8
	.byte	0
	.value	0
	.value	0
	.quad	.LFB94
	.quad	.LFE94-.LFB94
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
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LFB94
	.quad	.LFE94
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
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF27:
	.string	"aws_lc_0_38_0_OPENSSL_realloc"
.LASF39:
	.string	"aws_lc_0_38_0_BUF_strdup"
.LASF35:
	.string	"aws_lc_0_38_0_BUF_strlcpy"
.LASF37:
	.string	"aws_lc_0_38_0_BUF_strnlen"
.LASF14:
	.string	"data"
.LASF34:
	.string	"dst_size"
.LASF38:
	.string	"max_len"
.LASF8:
	.string	"short int"
.LASF11:
	.string	"size_t"
.LASF2:
	.string	"long long int"
.LASF25:
	.string	"memcpy"
.LASF45:
	.string	"alloc_size"
.LASF16:
	.string	"__int128"
.LASF52:
	.string	"aws_lc_0_38_0_BUF_MEM_new"
.LASF12:
	.string	"BUF_MEM"
.LASF29:
	.string	"aws_lc_0_38_0_OPENSSL_free"
.LASF36:
	.string	"aws_lc_0_38_0_BUF_strndup"
.LASF19:
	.string	"aws_lc_0_38_0_OPENSSL_memdup"
.LASF44:
	.string	"new_len"
.LASF53:
	.string	"OPENSSL_memset"
.LASF23:
	.string	"aws_lc_0_38_0_OPENSSL_strnlen"
.LASF13:
	.string	"length"
.LASF17:
	.string	"__int128 unsigned"
.LASF26:
	.string	"memset"
.LASF0:
	.string	"long int"
.LASF51:
	.string	"aws_lc_0_38_0_BUF_MEM_free"
.LASF28:
	.string	"aws_lc_0_38_0_ERR_put_error"
.LASF30:
	.string	"aws_lc_0_38_0_OPENSSL_zalloc"
.LASF47:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF3:
	.string	"long double"
.LASF50:
	.string	"buf_mem_st"
.LASF4:
	.string	"unsigned char"
.LASF32:
	.string	"aws_lc_0_38_0_BUF_memdup"
.LASF7:
	.string	"signed char"
.LASF33:
	.string	"aws_lc_0_38_0_BUF_strlcat"
.LASF10:
	.string	"long long unsigned int"
.LASF6:
	.string	"unsigned int"
.LASF5:
	.string	"short unsigned int"
.LASF9:
	.string	"char"
.LASF48:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/buf/buf.c"
.LASF46:
	.string	"new_buf"
.LASF42:
	.string	"aws_lc_0_38_0_BUF_MEM_grow"
.LASF18:
	.string	"_Bool"
.LASF1:
	.string	"long unsigned int"
.LASF43:
	.string	"aws_lc_0_38_0_BUF_MEM_reserve"
.LASF15:
	.string	"double"
.LASF24:
	.string	"aws_lc_0_38_0_OPENSSL_strdup"
.LASF20:
	.string	"aws_lc_0_38_0_OPENSSL_strlcat"
.LASF31:
	.string	"size"
.LASF21:
	.string	"aws_lc_0_38_0_OPENSSL_strlcpy"
.LASF22:
	.string	"aws_lc_0_38_0_OPENSSL_strndup"
.LASF49:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF41:
	.string	"aws_lc_0_38_0_BUF_MEM_grow_clean"
.LASF54:
	.string	"OPENSSL_memcpy"
.LASF40:
	.string	"aws_lc_0_38_0_BUF_MEM_append"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
