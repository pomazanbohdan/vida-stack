	.file	"unicode.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/unicode.c"
	.section	.text.is_valid_code_point,"ax",@progbits
	.type	is_valid_code_point, @function
is_valid_code_point:
.LFB0:
	.loc 1 20 44
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movl	%edi, -4(%rbp)
	.loc 1 22 6
	cmpl	$1114111, -4(%rbp)
	ja	.L2
	.loc 1 27 10
	movl	-4(%rbp), %eax
	andl	$65534, %eax
	.loc 1 23 20
	cmpl	$65534, %eax
	je	.L2
	.loc 1 27 30
	cmpl	$64975, -4(%rbp)
	jbe	.L3
	.loc 1 28 20
	cmpl	$65007, -4(%rbp)
	jbe	.L2
.L3:
	.loc 1 28 36 discriminator 1
	cmpl	$55295, -4(%rbp)
	jbe	.L4
	.loc 1 30 20
	cmpl	$57343, -4(%rbp)
	ja	.L4
.L2:
	.loc 1 31 12
	movl	$0, %eax
	jmp	.L5
.L4:
	.loc 1 33 10
	movl	$1, %eax
.L5:
	.loc 1 34 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE0:
	.size	is_valid_code_point, .-is_valid_code_point
	.section	.text.aws_lc_0_38_0_cbs_get_utf8,"ax",@progbits
	.globl	aws_lc_0_38_0_cbs_get_utf8
	.type	aws_lc_0_38_0_cbs_get_utf8, @function
aws_lc_0_38_0_cbs_get_utf8:
.LFB1:
	.loc 1 42 43
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	.loc 1 44 8
	leaq	-25(%rbp), %rdx
	movq	-40(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 44 6 discriminator 1
	testl	%eax, %eax
	jne	.L7
	.loc 1 45 12
	movl	$0, %eax
	jmp	.L20
.L7:
	.loc 1 47 9
	movzbl	-25(%rbp), %eax
	.loc 1 47 6
	testb	%al, %al
	js	.L9
	.loc 1 48 10
	movzbl	-25(%rbp), %eax
	movzbl	%al, %edx
	movq	-48(%rbp), %rax
	movl	%edx, (%rax)
	.loc 1 49 12
	movl	$1, %eax
	jmp	.L20
.L9:
	.loc 1 53 10
	movzbl	-25(%rbp), %eax
	movzbl	%al, %eax
	andl	$224, %eax
	.loc 1 53 6
	cmpl	$192, %eax
	jne	.L10
	.loc 1 54 11
	movzbl	-25(%rbp), %eax
	movzbl	%al, %eax
	.loc 1 54 7
	andl	$31, %eax
	movl	%eax, -4(%rbp)
	.loc 1 55 9
	movq	$1, -16(%rbp)
	.loc 1 56 17
	movl	$128, -8(%rbp)
	jmp	.L11
.L10:
	.loc 1 57 17
	movzbl	-25(%rbp), %eax
	movzbl	%al, %eax
	andl	$240, %eax
	.loc 1 57 13
	cmpl	$224, %eax
	jne	.L12
	.loc 1 58 11
	movzbl	-25(%rbp), %eax
	movzbl	%al, %eax
	.loc 1 58 7
	andl	$15, %eax
	movl	%eax, -4(%rbp)
	.loc 1 59 9
	movq	$2, -16(%rbp)
	.loc 1 60 17
	movl	$2048, -8(%rbp)
	jmp	.L11
.L12:
	.loc 1 61 17
	movzbl	-25(%rbp), %eax
	movzbl	%al, %eax
	andl	$248, %eax
	.loc 1 61 13
	cmpl	$240, %eax
	jne	.L13
	.loc 1 62 11
	movzbl	-25(%rbp), %eax
	movzbl	%al, %eax
	.loc 1 62 7
	andl	$7, %eax
	movl	%eax, -4(%rbp)
	.loc 1 63 9
	movq	$3, -16(%rbp)
	.loc 1 64 17
	movl	$65536, -8(%rbp)
	jmp	.L11
.L13:
	.loc 1 66 12
	movl	$0, %eax
	jmp	.L20
.L11:
.LBB2:
	.loc 1 68 15
	movq	$0, -24(%rbp)
	.loc 1 68 3
	jmp	.L14
.L17:
	.loc 1 69 10
	leaq	-25(%rbp), %rdx
	movq	-40(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 69 8 discriminator 1
	testl	%eax, %eax
	je	.L15
	.loc 1 70 12
	movzbl	-25(%rbp), %eax
	movzbl	%al, %eax
	andl	$192, %eax
	.loc 1 69 30 discriminator 1
	cmpl	$128, %eax
	je	.L16
.L15:
	.loc 1 71 14
	movl	$0, %eax
	jmp	.L20
.L16:
	.loc 1 73 7
	sall	$6, -4(%rbp)
	.loc 1 74 12
	movzbl	-25(%rbp), %eax
	movzbl	%al, %eax
	andl	$63, %eax
	.loc 1 74 7
	orl	%eax, -4(%rbp)
	.loc 1 68 32 discriminator 2
	addq	$1, -24(%rbp)
.L14:
	.loc 1 68 24 discriminator 1
	movq	-24(%rbp), %rax
	cmpq	-16(%rbp), %rax
	jb	.L17
.LBE2:
	.loc 1 76 8
	movl	-4(%rbp), %eax
	movl	%eax, %edi
	call	is_valid_code_point
	.loc 1 76 6 discriminator 1
	testl	%eax, %eax
	je	.L18
	.loc 1 76 31 discriminator 1
	movl	-4(%rbp), %eax
	cmpl	-8(%rbp), %eax
	jnb	.L19
.L18:
	.loc 1 78 12
	movl	$0, %eax
	jmp	.L20
.L19:
	.loc 1 80 8
	movq	-48(%rbp), %rax
	movl	-4(%rbp), %edx
	movl	%edx, (%rax)
	.loc 1 81 10
	movl	$1, %eax
.L20:
	.loc 1 82 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE1:
	.size	aws_lc_0_38_0_cbs_get_utf8, .-aws_lc_0_38_0_cbs_get_utf8
	.section	.text.aws_lc_0_38_0_cbs_get_latin1,"ax",@progbits
	.globl	aws_lc_0_38_0_cbs_get_latin1
	.type	aws_lc_0_38_0_cbs_get_latin1, @function
aws_lc_0_38_0_cbs_get_latin1:
.LFB2:
	.loc 1 84 45
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 86 8
	leaq	-1(%rbp), %rdx
	movq	-24(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u8@PLT
	.loc 1 86 6 discriminator 1
	testl	%eax, %eax
	jne	.L22
	.loc 1 87 12
	movl	$0, %eax
	jmp	.L24
.L22:
	.loc 1 89 8
	movzbl	-1(%rbp), %eax
	movzbl	%al, %edx
	movq	-32(%rbp), %rax
	movl	%edx, (%rax)
	.loc 1 90 10
	movl	$1, %eax
.L24:
	.loc 1 91 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE2:
	.size	aws_lc_0_38_0_cbs_get_latin1, .-aws_lc_0_38_0_cbs_get_latin1
	.section	.text.aws_lc_0_38_0_cbs_get_ucs2_be,"ax",@progbits
	.globl	aws_lc_0_38_0_cbs_get_ucs2_be
	.type	aws_lc_0_38_0_cbs_get_ucs2_be, @function
aws_lc_0_38_0_cbs_get_ucs2_be:
.LFB3:
	.loc 1 93 46
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 96 8
	leaq	-2(%rbp), %rdx
	movq	-24(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u16@PLT
	.loc 1 96 6 discriminator 1
	testl	%eax, %eax
	je	.L26
	.loc 1 97 8
	movzwl	-2(%rbp), %eax
	movzwl	%ax, %eax
	movl	%eax, %edi
	call	is_valid_code_point
	.loc 1 96 29 discriminator 1
	testl	%eax, %eax
	jne	.L27
.L26:
	.loc 1 98 12
	movl	$0, %eax
	jmp	.L29
.L27:
	.loc 1 100 8
	movzwl	-2(%rbp), %eax
	movzwl	%ax, %edx
	movq	-32(%rbp), %rax
	movl	%edx, (%rax)
	.loc 1 101 10
	movl	$1, %eax
.L29:
	.loc 1 102 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE3:
	.size	aws_lc_0_38_0_cbs_get_ucs2_be, .-aws_lc_0_38_0_cbs_get_ucs2_be
	.section	.text.aws_lc_0_38_0_cbs_get_utf32_be,"ax",@progbits
	.globl	aws_lc_0_38_0_cbs_get_utf32_be
	.type	aws_lc_0_38_0_cbs_get_utf32_be, @function
aws_lc_0_38_0_cbs_get_utf32_be:
.LFB4:
	.loc 1 104 47
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 1 105 10
	movq	-16(%rbp), %rdx
	movq	-8(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_u32@PLT
	.loc 1 105 32 discriminator 1
	testl	%eax, %eax
	je	.L31
	.loc 1 105 35 discriminator 1
	movq	-16(%rbp), %rax
	movl	(%rax), %eax
	movl	%eax, %edi
	call	is_valid_code_point
	.loc 1 105 32 discriminator 1
	testl	%eax, %eax
	je	.L31
	.loc 1 105 32 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 1 105 32
	jmp	.L33
.L31:
	.loc 1 105 32 discriminator 4
	movl	$0, %eax
.L33:
	.loc 1 106 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE4:
	.size	aws_lc_0_38_0_cbs_get_utf32_be, .-aws_lc_0_38_0_cbs_get_utf32_be
	.section	.text.aws_lc_0_38_0_cbb_get_utf8_len,"ax",@progbits
	.globl	aws_lc_0_38_0_cbb_get_utf8_len
	.type	aws_lc_0_38_0_cbb_get_utf8_len, @function
aws_lc_0_38_0_cbb_get_utf8_len:
.LFB5:
	.loc 1 108 37
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movl	%edi, -4(%rbp)
	.loc 1 109 6
	cmpl	$127, -4(%rbp)
	ja	.L35
	.loc 1 110 12
	movl	$1, %eax
	jmp	.L36
.L35:
	.loc 1 112 6
	cmpl	$2047, -4(%rbp)
	ja	.L37
	.loc 1 113 12
	movl	$2, %eax
	jmp	.L36
.L37:
	.loc 1 115 6
	cmpl	$65535, -4(%rbp)
	ja	.L38
	.loc 1 116 12
	movl	$3, %eax
	jmp	.L36
.L38:
	.loc 1 118 10
	movl	$4, %eax
.L36:
	.loc 1 119 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE5:
	.size	aws_lc_0_38_0_cbb_get_utf8_len, .-aws_lc_0_38_0_cbb_get_utf8_len
	.section	.text.aws_lc_0_38_0_cbb_add_utf8,"ax",@progbits
	.globl	aws_lc_0_38_0_cbb_add_utf8
	.type	aws_lc_0_38_0_cbb_add_utf8, @function
aws_lc_0_38_0_cbb_add_utf8:
.LFB6:
	.loc 1 121 40
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, -12(%rbp)
	.loc 1 122 8
	movl	-12(%rbp), %eax
	movl	%eax, %edi
	call	is_valid_code_point
	.loc 1 122 6 discriminator 1
	testl	%eax, %eax
	jne	.L40
	.loc 1 123 12
	movl	$0, %eax
	jmp	.L41
.L40:
	.loc 1 125 6
	cmpl	$127, -12(%rbp)
	ja	.L42
	.loc 1 126 28
	movl	-12(%rbp), %eax
	.loc 1 126 12
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	jmp	.L41
.L42:
	.loc 1 128 6
	cmpl	$2047, -12(%rbp)
	ja	.L43
	.loc 1 129 45
	movl	-12(%rbp), %eax
	shrl	$6, %eax
	.loc 1 129 40
	orl	$-64, %eax
	.loc 1 129 12
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 129 52 discriminator 1
	testl	%eax, %eax
	je	.L44
	.loc 1 130 45
	movl	-12(%rbp), %eax
	andl	$63, %eax
	.loc 1 130 40
	orl	$-128, %eax
	.loc 1 130 12
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 129 52 discriminator 1
	testl	%eax, %eax
	je	.L44
	.loc 1 129 52 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 1 129 52
	jmp	.L41
.L44:
	.loc 1 129 52 discriminator 4
	movl	$0, %eax
	.loc 1 129 52
	jmp	.L41
.L43:
	.loc 1 132 6 is_stmt 1
	cmpl	$65535, -12(%rbp)
	ja	.L46
	.loc 1 133 45
	movl	-12(%rbp), %eax
	shrl	$12, %eax
	.loc 1 133 40
	orl	$-32, %eax
	.loc 1 133 12
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 134 71
	testl	%eax, %eax
	je	.L47
	.loc 1 134 46 discriminator 1
	movl	-12(%rbp), %eax
	shrl	$6, %eax
	.loc 1 134 52 discriminator 1
	andl	$63, %eax
	.loc 1 134 40 discriminator 1
	orl	$-128, %eax
	.loc 1 134 12 discriminator 1
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 133 53
	testl	%eax, %eax
	je	.L47
	.loc 1 135 45
	movl	-12(%rbp), %eax
	andl	$63, %eax
	.loc 1 135 40
	orl	$-128, %eax
	.loc 1 135 12
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 134 71 discriminator 4
	testl	%eax, %eax
	je	.L47
	.loc 1 134 71 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 1 134 71
	jmp	.L41
.L47:
	.loc 1 134 71 discriminator 2
	movl	$0, %eax
	.loc 1 134 71
	jmp	.L41
.L46:
	.loc 1 137 6 is_stmt 1
	cmpl	$1114111, -12(%rbp)
	ja	.L49
	.loc 1 138 45
	movl	-12(%rbp), %eax
	shrl	$18, %eax
	.loc 1 138 40
	orl	$-16, %eax
	.loc 1 138 12
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 140 71
	testl	%eax, %eax
	je	.L50
	.loc 1 139 46
	movl	-12(%rbp), %eax
	shrl	$12, %eax
	.loc 1 139 53
	andl	$63, %eax
	.loc 1 139 40
	orl	$-128, %eax
	.loc 1 139 12
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 138 53
	testl	%eax, %eax
	je	.L50
	.loc 1 140 46
	movl	-12(%rbp), %eax
	shrl	$6, %eax
	.loc 1 140 52
	andl	$63, %eax
	.loc 1 140 40
	orl	$-128, %eax
	.loc 1 140 12
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 139 72
	testl	%eax, %eax
	je	.L50
	.loc 1 141 45
	movl	-12(%rbp), %eax
	andl	$63, %eax
	.loc 1 141 40
	orl	$-128, %eax
	.loc 1 141 12
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 140 71 discriminator 3
	testl	%eax, %eax
	je	.L50
	.loc 1 140 71 is_stmt 0 discriminator 2
	movl	$1, %eax
	.loc 1 140 71
	jmp	.L41
.L50:
	.loc 1 140 71 discriminator 1
	movl	$0, %eax
	.loc 1 140 71
	jmp	.L41
.L49:
	.loc 1 143 10 is_stmt 1
	movl	$0, %eax
.L41:
	.loc 1 144 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE6:
	.size	aws_lc_0_38_0_cbb_add_utf8, .-aws_lc_0_38_0_cbb_add_utf8
	.section	.text.aws_lc_0_38_0_cbb_add_latin1,"ax",@progbits
	.globl	aws_lc_0_38_0_cbb_add_latin1
	.type	aws_lc_0_38_0_cbb_add_latin1, @function
aws_lc_0_38_0_cbb_add_latin1:
.LFB7:
	.loc 1 146 42
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, -12(%rbp)
	.loc 1 147 20
	cmpl	$255, -12(%rbp)
	ja	.L53
	.loc 1 147 39 discriminator 1
	movl	-12(%rbp), %eax
	.loc 1 147 23 discriminator 1
	movzbl	%al, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u8@PLT
	.loc 1 147 20 discriminator 1
	testl	%eax, %eax
	je	.L53
	.loc 1 147 20 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 1 147 20
	jmp	.L55
.L53:
	.loc 1 147 20 discriminator 4
	movl	$0, %eax
.L55:
	.loc 1 148 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE7:
	.size	aws_lc_0_38_0_cbb_add_latin1, .-aws_lc_0_38_0_cbb_add_latin1
	.section	.text.aws_lc_0_38_0_cbb_add_ucs2_be,"ax",@progbits
	.globl	aws_lc_0_38_0_cbb_add_ucs2_be
	.type	aws_lc_0_38_0_cbb_add_ucs2_be, @function
aws_lc_0_38_0_cbb_add_ucs2_be:
.LFB8:
	.loc 1 150 43
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, -12(%rbp)
	.loc 1 151 48
	cmpl	$65535, -12(%rbp)
	ja	.L57
	.loc 1 151 25 discriminator 1
	movl	-12(%rbp), %eax
	movl	%eax, %edi
	call	is_valid_code_point
	.loc 1 151 22 discriminator 1
	testl	%eax, %eax
	je	.L57
	.loc 1 151 68 discriminator 3
	movl	-12(%rbp), %eax
	.loc 1 151 51 discriminator 3
	movzwl	%ax, %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u16@PLT
	.loc 1 151 48 discriminator 1
	testl	%eax, %eax
	je	.L57
	.loc 1 151 48 is_stmt 0 discriminator 5
	movl	$1, %eax
	.loc 1 151 48
	jmp	.L59
.L57:
	.loc 1 151 48 discriminator 6
	movl	$0, %eax
.L59:
	.loc 1 152 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE8:
	.size	aws_lc_0_38_0_cbb_add_ucs2_be, .-aws_lc_0_38_0_cbb_add_ucs2_be
	.section	.text.aws_lc_0_38_0_cbb_add_utf32_be,"ax",@progbits
	.globl	aws_lc_0_38_0_cbb_add_utf32_be
	.type	aws_lc_0_38_0_cbb_add_utf32_be, @function
aws_lc_0_38_0_cbb_add_utf32_be:
.LFB9:
	.loc 1 154 44
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, -12(%rbp)
	.loc 1 155 10
	movl	-12(%rbp), %eax
	movl	%eax, %edi
	call	is_valid_code_point
	.loc 1 155 33 discriminator 1
	testl	%eax, %eax
	je	.L61
	.loc 1 155 36 discriminator 1
	movl	-12(%rbp), %edx
	movq	-8(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_u32@PLT
	.loc 1 155 33 discriminator 1
	testl	%eax, %eax
	je	.L61
	.loc 1 155 33 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 1 155 33
	jmp	.L63
.L61:
	.loc 1 155 33 discriminator 4
	movl	$0, %eax
.L63:
	.loc 1 156 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE9:
	.size	aws_lc_0_38_0_cbb_add_utf32_be, .-aws_lc_0_38_0_cbb_add_utf32_be
	.text
.Letext0:
	.file 2 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 3 "/usr/include/bits/types.h"
	.file 4 "/usr/include/bits/stdint-uintn.h"
	.file 5 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/bytestring.h"
	.file 6 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x5b2
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
	.long	.LASF8
	.byte	0x2
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
	.byte	0x3
	.byte	0x26
	.byte	0x17
	.long	0x58
	.uleb128 0x2
	.byte	0x2
	.byte	0x5
	.long	.LASF10
	.uleb128 0x3
	.long	.LASF11
	.byte	0x3
	.byte	0x28
	.byte	0x1c
	.long	0x5f
	.uleb128 0x3
	.long	.LASF12
	.byte	0x3
	.byte	0x2a
	.byte	0x16
	.long	0x66
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF13
	.uleb128 0x3
	.long	.LASF14
	.byte	0x4
	.byte	0x18
	.byte	0x13
	.long	0x74
	.uleb128 0x5
	.long	0xa6
	.uleb128 0x3
	.long	.LASF15
	.byte	0x4
	.byte	0x19
	.byte	0x14
	.long	0x87
	.uleb128 0x3
	.long	.LASF16
	.byte	0x4
	.byte	0x1a
	.byte	0x14
	.long	0x93
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF17
	.uleb128 0x6
	.string	"CBB"
	.byte	0x6
	.value	0x194
	.byte	0x17
	.long	0xe3
	.uleb128 0x7
	.long	.LASF20
	.byte	0x30
	.byte	0x5
	.value	0x1be
	.byte	0x8
	.long	0x11a
	.uleb128 0x8
	.long	.LASF18
	.byte	0x5
	.value	0x1c0
	.byte	0x8
	.long	0x22b
	.byte	0
	.uleb128 0x8
	.long	.LASF19
	.byte	0x5
	.value	0x1c3
	.byte	0x8
	.long	0x9f
	.byte	0x8
	.uleb128 0x9
	.string	"u"
	.byte	0x5
	.value	0x1c7
	.byte	0x5
	.long	0x206
	.byte	0x10
	.byte	0
	.uleb128 0x6
	.string	"CBS"
	.byte	0x6
	.value	0x195
	.byte	0x17
	.long	0x127
	.uleb128 0xa
	.long	.LASF21
	.byte	0x10
	.byte	0x5
	.byte	0x28
	.byte	0x8
	.long	0x14f
	.uleb128 0xb
	.long	.LASF22
	.byte	0x5
	.byte	0x29
	.byte	0x12
	.long	0x14f
	.byte	0
	.uleb128 0xc
	.string	"len"
	.byte	0x5
	.byte	0x2a
	.byte	0xa
	.long	0x30
	.byte	0x8
	.byte	0
	.uleb128 0xd
	.byte	0x8
	.long	0xb2
	.uleb128 0x7
	.long	.LASF23
	.byte	0x20
	.byte	0x5
	.value	0x1a4
	.byte	0x8
	.long	0x1b0
	.uleb128 0x9
	.string	"buf"
	.byte	0x5
	.value	0x1a5
	.byte	0xc
	.long	0x1b0
	.byte	0
	.uleb128 0x9
	.string	"len"
	.byte	0x5
	.value	0x1a7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0x9
	.string	"cap"
	.byte	0x5
	.value	0x1a9
	.byte	0xa
	.long	0x30
	.byte	0x10
	.uleb128 0xe
	.long	.LASF24
	.byte	0x5
	.value	0x1ac
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x1f
	.byte	0x18
	.uleb128 0xe
	.long	.LASF25
	.byte	0x5
	.value	0x1af
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x1e
	.byte	0x18
	.byte	0
	.uleb128 0xd
	.byte	0x8
	.long	0xa6
	.uleb128 0x7
	.long	.LASF26
	.byte	0x18
	.byte	0x5
	.value	0x1b2
	.byte	0x8
	.long	0x200
	.uleb128 0x8
	.long	.LASF27
	.byte	0x5
	.value	0x1b4
	.byte	0x19
	.long	0x200
	.byte	0
	.uleb128 0x8
	.long	.LASF28
	.byte	0x5
	.value	0x1b7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0x8
	.long	.LASF29
	.byte	0x5
	.value	0x1ba
	.byte	0xb
	.long	0xa6
	.byte	0x10
	.uleb128 0xe
	.long	.LASF30
	.byte	0x5
	.value	0x1bb
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x17
	.byte	0x10
	.byte	0
	.uleb128 0xd
	.byte	0x8
	.long	0x155
	.uleb128 0xf
	.byte	0x20
	.byte	0x5
	.value	0x1c4
	.byte	0x3
	.long	0x22b
	.uleb128 0x10
	.long	.LASF27
	.byte	0x5
	.value	0x1c5
	.byte	0x1a
	.long	0x155
	.uleb128 0x10
	.long	.LASF18
	.byte	0x5
	.value	0x1c6
	.byte	0x19
	.long	0x1b6
	.byte	0
	.uleb128 0xd
	.byte	0x8
	.long	0xd6
	.uleb128 0x11
	.long	.LASF31
	.byte	0x5
	.value	0x242
	.byte	0x14
	.long	0x43
	.long	0x24d
	.uleb128 0x12
	.long	0x22b
	.uleb128 0x12
	.long	0xc3
	.byte	0
	.uleb128 0x11
	.long	.LASF32
	.byte	0x5
	.value	0x236
	.byte	0x14
	.long	0x43
	.long	0x269
	.uleb128 0x12
	.long	0x22b
	.uleb128 0x12
	.long	0xb7
	.byte	0
	.uleb128 0x11
	.long	.LASF33
	.byte	0x5
	.value	0x232
	.byte	0x14
	.long	0x43
	.long	0x285
	.uleb128 0x12
	.long	0x22b
	.uleb128 0x12
	.long	0xa6
	.byte	0
	.uleb128 0x13
	.long	.LASF34
	.byte	0x5
	.byte	0x75
	.byte	0x14
	.long	0x43
	.long	0x2a0
	.uleb128 0x12
	.long	0x2a0
	.uleb128 0x12
	.long	0x2a6
	.byte	0
	.uleb128 0xd
	.byte	0x8
	.long	0x11a
	.uleb128 0xd
	.byte	0x8
	.long	0xc3
	.uleb128 0x13
	.long	.LASF35
	.byte	0x5
	.byte	0x69
	.byte	0x14
	.long	0x43
	.long	0x2c7
	.uleb128 0x12
	.long	0x2a0
	.uleb128 0x12
	.long	0x2c7
	.byte	0
	.uleb128 0xd
	.byte	0x8
	.long	0xb7
	.uleb128 0x13
	.long	.LASF36
	.byte	0x5
	.byte	0x65
	.byte	0x14
	.long	0x43
	.long	0x2e8
	.uleb128 0x12
	.long	0x2a0
	.uleb128 0x12
	.long	0x1b0
	.byte	0
	.uleb128 0x14
	.long	.LASF37
	.byte	0x1
	.byte	0x9a
	.byte	0x5
	.long	0x43
	.quad	.LFB9
	.quad	.LFE9-.LFB9
	.uleb128 0x1
	.byte	0x9c
	.long	0x327
	.uleb128 0x15
	.string	"cbb"
	.byte	0x1
	.byte	0x9a
	.byte	0x1b
	.long	0x22b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x15
	.string	"u"
	.byte	0x1
	.byte	0x9a
	.byte	0x29
	.long	0xc3
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x14
	.long	.LASF38
	.byte	0x1
	.byte	0x96
	.byte	0x5
	.long	0x43
	.quad	.LFB8
	.quad	.LFE8-.LFB8
	.uleb128 0x1
	.byte	0x9c
	.long	0x366
	.uleb128 0x15
	.string	"cbb"
	.byte	0x1
	.byte	0x96
	.byte	0x1a
	.long	0x22b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x15
	.string	"u"
	.byte	0x1
	.byte	0x96
	.byte	0x28
	.long	0xc3
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x14
	.long	.LASF39
	.byte	0x1
	.byte	0x92
	.byte	0x5
	.long	0x43
	.quad	.LFB7
	.quad	.LFE7-.LFB7
	.uleb128 0x1
	.byte	0x9c
	.long	0x3a5
	.uleb128 0x15
	.string	"cbb"
	.byte	0x1
	.byte	0x92
	.byte	0x19
	.long	0x22b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x15
	.string	"u"
	.byte	0x1
	.byte	0x92
	.byte	0x27
	.long	0xc3
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x14
	.long	.LASF40
	.byte	0x1
	.byte	0x79
	.byte	0x5
	.long	0x43
	.quad	.LFB6
	.quad	.LFE6-.LFB6
	.uleb128 0x1
	.byte	0x9c
	.long	0x3e4
	.uleb128 0x15
	.string	"cbb"
	.byte	0x1
	.byte	0x79
	.byte	0x17
	.long	0x22b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x15
	.string	"u"
	.byte	0x1
	.byte	0x79
	.byte	0x25
	.long	0xc3
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x16
	.long	.LASF41
	.byte	0x1
	.byte	0x6c
	.byte	0x8
	.long	0x30
	.quad	.LFB5
	.quad	.LFE5-.LFB5
	.uleb128 0x1
	.byte	0x9c
	.long	0x414
	.uleb128 0x15
	.string	"u"
	.byte	0x1
	.byte	0x6c
	.byte	0x22
	.long	0xc3
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.uleb128 0x14
	.long	.LASF42
	.byte	0x1
	.byte	0x68
	.byte	0x5
	.long	0x43
	.quad	.LFB4
	.quad	.LFE4-.LFB4
	.uleb128 0x1
	.byte	0x9c
	.long	0x455
	.uleb128 0x15
	.string	"cbs"
	.byte	0x1
	.byte	0x68
	.byte	0x1b
	.long	0x2a0
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x15
	.string	"out"
	.byte	0x1
	.byte	0x68
	.byte	0x2a
	.long	0x2a6
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x14
	.long	.LASF43
	.byte	0x1
	.byte	0x5d
	.byte	0x5
	.long	0x43
	.quad	.LFB3
	.quad	.LFE3-.LFB3
	.uleb128 0x1
	.byte	0x9c
	.long	0x4a3
	.uleb128 0x15
	.string	"cbs"
	.byte	0x1
	.byte	0x5d
	.byte	0x1a
	.long	0x2a0
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x15
	.string	"out"
	.byte	0x1
	.byte	0x5d
	.byte	0x29
	.long	0x2a6
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x17
	.string	"c"
	.byte	0x1
	.byte	0x5f
	.byte	0xc
	.long	0xb7
	.uleb128 0x2
	.byte	0x91
	.sleb128 -18
	.byte	0
	.uleb128 0x14
	.long	.LASF44
	.byte	0x1
	.byte	0x54
	.byte	0x5
	.long	0x43
	.quad	.LFB2
	.quad	.LFE2-.LFB2
	.uleb128 0x1
	.byte	0x9c
	.long	0x4f1
	.uleb128 0x15
	.string	"cbs"
	.byte	0x1
	.byte	0x54
	.byte	0x19
	.long	0x2a0
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x15
	.string	"out"
	.byte	0x1
	.byte	0x54
	.byte	0x28
	.long	0x2a6
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x17
	.string	"c"
	.byte	0x1
	.byte	0x55
	.byte	0xb
	.long	0xa6
	.uleb128 0x2
	.byte	0x91
	.sleb128 -17
	.byte	0
	.uleb128 0x14
	.long	.LASF45
	.byte	0x1
	.byte	0x2a
	.byte	0x5
	.long	0x43
	.quad	.LFB1
	.quad	.LFE1-.LFB1
	.uleb128 0x1
	.byte	0x9c
	.long	0x589
	.uleb128 0x15
	.string	"cbs"
	.byte	0x1
	.byte	0x2a
	.byte	0x17
	.long	0x2a0
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x15
	.string	"out"
	.byte	0x1
	.byte	0x2a
	.byte	0x26
	.long	0x2a6
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x17
	.string	"c"
	.byte	0x1
	.byte	0x2b
	.byte	0xb
	.long	0xa6
	.uleb128 0x2
	.byte	0x91
	.sleb128 -41
	.uleb128 0x17
	.string	"v"
	.byte	0x1
	.byte	0x33
	.byte	0xc
	.long	0xc3
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x18
	.long	.LASF46
	.byte	0x1
	.byte	0x33
	.byte	0xf
	.long	0xc3
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x17
	.string	"len"
	.byte	0x1
	.byte	0x34
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x19
	.quad	.LBB2
	.quad	.LBE2-.LBB2
	.uleb128 0x17
	.string	"i"
	.byte	0x1
	.byte	0x44
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.byte	0
	.uleb128 0x1a
	.long	.LASF50
	.byte	0x1
	.byte	0x14
	.byte	0xc
	.long	0x43
	.quad	.LFB0
	.quad	.LFE0-.LFB0
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x15
	.string	"v"
	.byte	0x1
	.byte	0x14
	.byte	0x29
	.long	0xc3
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
	.uleb128 0x26
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x6
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
	.uleb128 0x7
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
	.uleb128 0x8
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
	.uleb128 0x9
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
	.uleb128 0xf
	.byte	0
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x49
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
	.uleb128 0xf
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
	.uleb128 0x10
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
	.uleb128 0x12
	.uleb128 0x5
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x13
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
	.uleb128 0x14
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
	.uleb128 0x15
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
	.uleb128 0x17
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
	.uleb128 0x18
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
	.uleb128 0x19
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
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
	.byte	0
	.byte	0
	.byte	0
	.section	.debug_aranges,"",@progbits
	.long	0xbc
	.value	0x2
	.long	.Ldebug_info0
	.byte	0x8
	.byte	0
	.value	0
	.value	0
	.quad	.LFB0
	.quad	.LFE0-.LFB0
	.quad	.LFB1
	.quad	.LFE1-.LFB1
	.quad	.LFB2
	.quad	.LFE2-.LFB2
	.quad	.LFB3
	.quad	.LFE3-.LFB3
	.quad	.LFB4
	.quad	.LFE4-.LFB4
	.quad	.LFB5
	.quad	.LFE5-.LFB5
	.quad	.LFB6
	.quad	.LFE6-.LFB6
	.quad	.LFB7
	.quad	.LFE7-.LFB7
	.quad	.LFB8
	.quad	.LFE8-.LFB8
	.quad	.LFB9
	.quad	.LFE9-.LFB9
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LFB0
	.quad	.LFE0
	.quad	.LFB1
	.quad	.LFE1
	.quad	.LFB2
	.quad	.LFE2
	.quad	.LFB3
	.quad	.LFE3
	.quad	.LFB4
	.quad	.LFE4
	.quad	.LFB5
	.quad	.LFE5
	.quad	.LFB6
	.quad	.LFE6
	.quad	.LFB7
	.quad	.LFE7
	.quad	.LFB8
	.quad	.LFE8
	.quad	.LFB9
	.quad	.LFE9
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF29:
	.string	"pending_len_len"
.LASF34:
	.string	"aws_lc_0_38_0_CBS_get_u32"
.LASF20:
	.string	"cbb_st"
.LASF32:
	.string	"aws_lc_0_38_0_CBB_add_u16"
.LASF24:
	.string	"can_resize"
.LASF10:
	.string	"short int"
.LASF8:
	.string	"size_t"
.LASF12:
	.string	"__uint32_t"
.LASF26:
	.string	"cbb_child_st"
.LASF11:
	.string	"__uint16_t"
.LASF44:
	.string	"aws_lc_0_38_0_cbs_get_latin1"
.LASF14:
	.string	"uint8_t"
.LASF19:
	.string	"is_child"
.LASF37:
	.string	"aws_lc_0_38_0_cbb_add_utf32_be"
.LASF42:
	.string	"aws_lc_0_38_0_cbs_get_utf32_be"
.LASF2:
	.string	"long long int"
.LASF38:
	.string	"aws_lc_0_38_0_cbb_add_ucs2_be"
.LASF45:
	.string	"aws_lc_0_38_0_cbs_get_utf8"
.LASF41:
	.string	"aws_lc_0_38_0_cbb_get_utf8_len"
.LASF28:
	.string	"offset"
.LASF9:
	.string	"__uint8_t"
.LASF47:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF3:
	.string	"long double"
.LASF21:
	.string	"cbs_st"
.LASF4:
	.string	"unsigned char"
.LASF43:
	.string	"aws_lc_0_38_0_cbs_get_ucs2_be"
.LASF7:
	.string	"signed char"
.LASF17:
	.string	"long long unsigned int"
.LASF48:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/unicode.c"
.LASF16:
	.string	"uint32_t"
.LASF6:
	.string	"unsigned int"
.LASF15:
	.string	"uint16_t"
.LASF31:
	.string	"aws_lc_0_38_0_CBB_add_u32"
.LASF23:
	.string	"cbb_buffer_st"
.LASF33:
	.string	"aws_lc_0_38_0_CBB_add_u8"
.LASF5:
	.string	"short unsigned int"
.LASF13:
	.string	"char"
.LASF0:
	.string	"long int"
.LASF30:
	.string	"pending_is_asn1"
.LASF39:
	.string	"aws_lc_0_38_0_cbb_add_latin1"
.LASF50:
	.string	"is_valid_code_point"
.LASF22:
	.string	"data"
.LASF1:
	.string	"long unsigned int"
.LASF18:
	.string	"child"
.LASF46:
	.string	"lower_bound"
.LASF35:
	.string	"aws_lc_0_38_0_CBS_get_u16"
.LASF49:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF25:
	.string	"error"
.LASF36:
	.string	"aws_lc_0_38_0_CBS_get_u8"
.LASF27:
	.string	"base"
.LASF40:
	.string	"aws_lc_0_38_0_cbb_add_utf8"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
