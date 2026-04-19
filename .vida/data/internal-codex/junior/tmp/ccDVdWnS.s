	.file	"ber.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/ber.c"
	.section	.rodata.kMaxDepth,"a"
	.align 4
	.type	kMaxDepth, @object
	.size	kMaxDepth, 4
kMaxDepth:
	.long	128
	.section	.text.is_string_type,"ax",@progbits
	.type	is_string_type, @function
is_string_type:
.LFB0:
	.loc 1 28 45
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movl	%edi, -4(%rbp)
	.loc 1 32 15
	movl	-4(%rbp), %eax
	andl	$-536870913, %eax
	cmpl	$30, %eax
	seta	%dl
	.loc 1 32 3
	testb	%dl, %dl
	jne	.L2
	movl	$1585188880, %edx
	shrx	%rax, %rdx, %rax
	andl	$1, %eax
	testq	%rax, %rax
	setne	%al
	testb	%al, %al
	je	.L2
	.loc 1 45 14
	movl	$1, %eax
	jmp	.L3
.L2:
	.loc 1 47 14
	movl	$0, %eax
.L3:
	.loc 1 49 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE0:
	.size	is_string_type, .-is_string_type
	.section	.text.cbs_find_ber,"ax",@progbits
	.type	cbs_find_ber, @function
cbs_find_ber:
.LFB1:
	.loc 1 55 77
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
	.loc 1 56 13
	movl	$128, %eax
	.loc 1 56 6
	cmpl	-84(%rbp), %eax
	jnb	.L5
	.loc 1 57 12
	movl	$0, %eax
	jmp	.L16
.L5:
	.loc 1 60 7
	movq	-72(%rbp), %rax
	movq	8(%rax), %rdx
	movq	(%rax), %rax
	movq	%rax, -16(%rbp)
	movq	%rdx, -8(%rbp)
	.loc 1 61 14
	movq	-80(%rbp), %rax
	movl	$0, (%rax)
	.loc 1 63 9
	jmp	.L7
.L15:
.LBB2:
	.loc 1 68 10
	leaq	-52(%rbp), %r8
	movq	-80(%rbp), %rdi
	leaq	-48(%rbp), %rcx
	leaq	-36(%rbp), %rdx
	leaq	-32(%rbp), %rsi
	leaq	-16(%rbp), %rax
	movq	%r8, %r9
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_any_ber_asn1_element@PLT
	.loc 1 68 8 discriminator 1
	testl	%eax, %eax
	jne	.L8
	.loc 1 70 14
	movl	$0, %eax
	jmp	.L16
.L8:
	.loc 1 72 9
	movq	-80(%rbp), %rax
	movl	(%rax), %eax
	.loc 1 72 8
	testl	%eax, %eax
	je	.L10
	.loc 1 73 14
	movl	$1, %eax
	jmp	.L16
.L10:
	.loc 1 75 13
	movl	-36(%rbp), %eax
	andl	$536870912, %eax
	.loc 1 75 8
	testl	%eax, %eax
	je	.L7
	.loc 1 76 11
	movl	-36(%rbp), %eax
	movl	%eax, %edi
	call	is_string_type
	.loc 1 76 10 discriminator 1
	testl	%eax, %eax
	je	.L12
	.loc 1 78 20
	movq	-80(%rbp), %rax
	movl	$1, (%rax)
	.loc 1 79 16
	movl	$1, %eax
	jmp	.L16
.L12:
	.loc 1 81 12
	movq	-48(%rbp), %rdx
	leaq	-32(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_skip@PLT
	.loc 1 81 10 discriminator 1
	testl	%eax, %eax
	je	.L13
	.loc 1 82 12
	movl	-84(%rbp), %eax
	leal	1(%rax), %edx
	movq	-80(%rbp), %rcx
	leaq	-32(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_find_ber
	.loc 1 81 44 discriminator 1
	testl	%eax, %eax
	jne	.L14
.L13:
	.loc 1 83 16
	movl	$0, %eax
	jmp	.L16
.L14:
	.loc 1 85 11
	movq	-80(%rbp), %rax
	movl	(%rax), %eax
	.loc 1 85 10
	testl	%eax, %eax
	je	.L7
	.loc 1 87 16
	movl	$1, %eax
	jmp	.L16
.L7:
.LBE2:
	.loc 1 63 10
	leaq	-16(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 63 23 discriminator 1
	testq	%rax, %rax
	jne	.L15
	.loc 1 92 10
	movl	$1, %eax
.L16:
	.loc 1 93 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE1:
	.size	cbs_find_ber, .-cbs_find_ber
	.section	.text.cbs_get_eoc,"ax",@progbits
	.type	cbs_get_eoc, @function
cbs_get_eoc:
.LFB2:
	.loc 1 97 34
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	.loc 1 98 7
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 98 6 discriminator 1
	cmpq	$1, %rax
	jbe	.L18
	.loc 1 99 20
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	.loc 1 99 20 is_stmt 0 discriminator 1
	movzbl	(%rax), %eax
	.loc 1 98 25 is_stmt 1 discriminator 1
	testb	%al, %al
	jne	.L18
	.loc 1 99 32
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	.loc 1 99 45 discriminator 1
	addq	$1, %rax
	movzbl	(%rax), %eax
	.loc 1 99 29 discriminator 1
	testb	%al, %al
	jne	.L18
	.loc 1 100 12
	movq	-8(%rbp), %rax
	movl	$2, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_skip@PLT
	jmp	.L19
.L18:
	.loc 1 102 10
	movl	$0, %eax
.L19:
	.loc 1 103 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE2:
	.size	cbs_get_eoc, .-cbs_get_eoc
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/ber.c"
	.align 8
.LC1:
	.string	"!(string_tag & CBS_ASN1_CONSTRUCTED)"
	.section	.text.cbs_convert_ber,"ax",@progbits
	.type	cbs_convert_ber, @function
cbs_convert_ber:
.LFB3:
	.loc 1 113 65
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%rbx
	subq	$168, %rsp
	.cfi_offset 3, -24
	movq	%rdi, -152(%rbp)
	movq	%rsi, -160(%rbp)
	movl	%edx, -164(%rbp)
	movl	%ecx, -168(%rbp)
	movl	%r8d, -172(%rbp)
	.loc 1 114 3
	movl	-164(%rbp), %eax
	andl	$536870912, %eax
	testl	%eax, %eax
	je	.L21
	.loc 1 114 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.1(%rip), %rax
	movq	%rax, %rcx
	movl	$114, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC1(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L21:
	.loc 1 116 13 is_stmt 1
	movl	$128, %eax
	.loc 1 116 6
	cmpl	-172(%rbp), %eax
	jnb	.L24
	.loc 1 117 12
	movl	$0, %eax
	jmp	.L23
.L41:
.LBB3:
	.loc 1 121 8
	cmpl	$0, -168(%rbp)
	je	.L25
	.loc 1 121 28 discriminator 1
	movq	-152(%rbp), %rax
	movq	%rax, %rdi
	call	cbs_get_eoc
	.loc 1 121 25 discriminator 1
	testl	%eax, %eax
	je	.L25
	.loc 1 122 14
	movl	$1, %eax
	jmp	.L23
.L25:
	.loc 1 126 23
	movl	-164(%rbp), %eax
	movl	%eax, -20(%rbp)
	.loc 1 130 10
	leaq	-84(%rbp), %rdi
	leaq	-80(%rbp), %rcx
	leaq	-68(%rbp), %rdx
	leaq	-64(%rbp), %rsi
	movq	-152(%rbp), %rax
	movq	%rdi, %r9
	movl	$0, %r8d
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_any_ber_asn1_element@PLT
	.loc 1 130 8 discriminator 1
	testl	%eax, %eax
	jne	.L27
	.loc 1 132 14
	movl	$0, %eax
	jmp	.L23
.L27:
	.loc 1 135 8
	cmpl	$0, -164(%rbp)
	je	.L28
	.loc 1 139 16
	movl	-68(%rbp), %eax
	andl	$-536870913, %eax
	.loc 1 139 10
	cmpl	%eax, -164(%rbp)
	je	.L29
	.loc 1 140 16
	movl	$0, %eax
	jmp	.L23
.L29:
	.loc 1 142 20
	movq	-160(%rbp), %rax
	movq	%rax, -32(%rbp)
	jmp	.L30
.L28:
.LBB4:
	.loc 1 144 20
	movl	-68(%rbp), %eax
	movl	%eax, -36(%rbp)
	.loc 1 145 16
	movl	-68(%rbp), %eax
	andl	$536870912, %eax
	.loc 1 145 10
	testl	%eax, %eax
	je	.L31
	.loc 1 145 43 discriminator 1
	movl	-68(%rbp), %eax
	movl	%eax, %edi
	call	is_string_type
	.loc 1 145 40 discriminator 1
	testl	%eax, %eax
	je	.L31
	.loc 1 148 17
	andl	$-536870913, -36(%rbp)
	.loc 1 149 26
	movl	-36(%rbp), %eax
	movl	%eax, -20(%rbp)
.L31:
	.loc 1 151 12
	movl	-36(%rbp), %edx
	leaq	-144(%rbp), %rcx
	movq	-160(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_asn1@PLT
	.loc 1 151 10 discriminator 1
	testl	%eax, %eax
	jne	.L32
	.loc 1 152 16
	movl	$0, %eax
	jmp	.L23
.L32:
	.loc 1 154 20
	leaq	-144(%rbp), %rax
	movq	%rax, -32(%rbp)
.L30:
.LBE4:
	.loc 1 157 9
	movl	-84(%rbp), %eax
	.loc 1 157 8
	testl	%eax, %eax
	je	.L33
	.loc 1 158 12
	movl	-172(%rbp), %eax
	leal	1(%rax), %ecx
	movl	-20(%rbp), %edx
	movq	-32(%rbp), %rsi
	movq	-152(%rbp), %rax
	movl	%ecx, %r8d
	movl	$1, %ecx
	movq	%rax, %rdi
	call	cbs_convert_ber
	.loc 1 158 10 discriminator 1
	testl	%eax, %eax
	je	.L34
	.loc 1 160 12
	movq	-160(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 159 62
	testl	%eax, %eax
	jne	.L42
.L34:
	.loc 1 161 16
	movl	$0, %eax
	jmp	.L23
.L33:
	.loc 1 166 10
	movq	-80(%rbp), %rdx
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_skip@PLT
	.loc 1 166 8 discriminator 1
	testl	%eax, %eax
	jne	.L37
	.loc 1 167 14
	movl	$0, %eax
	jmp	.L23
.L37:
	.loc 1 170 13
	movl	-68(%rbp), %eax
	andl	$536870912, %eax
	.loc 1 170 8
	testl	%eax, %eax
	je	.L38
	.loc 1 172 12
	movl	-172(%rbp), %eax
	leal	1(%rax), %ecx
	movl	-20(%rbp), %edx
	movq	-32(%rbp), %rsi
	leaq	-64(%rbp), %rax
	movl	%ecx, %r8d
	movl	$0, %ecx
	movq	%rax, %rdi
	call	cbs_convert_ber
	.loc 1 172 10 discriminator 1
	testl	%eax, %eax
	jne	.L39
	.loc 1 174 16
	movl	$0, %eax
	jmp	.L23
.L38:
	.loc 1 178 12
	leaq	-64(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, %rbx
	.loc 1 178 12 is_stmt 0 discriminator 1
	leaq	-64(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	movq	%rax, %rcx
	.loc 1 178 12 discriminator 2
	movq	-32(%rbp), %rax
	movq	%rbx, %rdx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_bytes@PLT
	.loc 1 178 10 is_stmt 1 discriminator 3
	testl	%eax, %eax
	jne	.L39
	.loc 1 180 16
	movl	$0, %eax
	jmp	.L23
.L39:
	.loc 1 184 10
	movq	-160(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_flush@PLT
	.loc 1 184 8 discriminator 1
	testl	%eax, %eax
	jne	.L24
	.loc 1 185 14
	movl	$0, %eax
	jmp	.L23
.L42:
	.loc 1 163 7
	nop
.L24:
.LBE3:
	.loc 1 120 10
	movq	-152(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 120 22 discriminator 1
	testq	%rax, %rax
	jne	.L41
	.loc 1 189 26
	cmpl	$0, -168(%rbp)
	sete	%al
	movzbl	%al, %eax
.L23:
	.loc 1 190 1
	movq	-8(%rbp), %rbx
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE3:
	.size	cbs_convert_ber, .-cbs_convert_ber
	.section	.text.aws_lc_0_38_0_CBS_asn1_ber_to_der,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_asn1_ber_to_der
	.type	aws_lc_0_38_0_CBS_asn1_ber_to_der, @function
aws_lc_0_38_0_CBS_asn1_ber_to_der:
.LFB4:
	.loc 1 192 67
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$96, %rsp
	movq	%rdi, -72(%rbp)
	movq	%rsi, -80(%rbp)
	movq	%rdx, -88(%rbp)
	.loc 1 198 8
	leaq	-52(%rbp), %rcx
	movq	-72(%rbp), %rax
	movl	$0, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	cbs_find_ber
	.loc 1 198 6 discriminator 1
	testl	%eax, %eax
	jne	.L44
	.loc 1 199 12
	movl	$0, %eax
	jmp	.L50
.L44:
	.loc 1 202 7
	movl	-52(%rbp), %eax
	.loc 1 202 6
	testl	%eax, %eax
	jne	.L46
	.loc 1 203 10
	movq	-80(%rbp), %rsi
	movq	-72(%rbp), %rax
	movl	$0, %ecx
	movl	$0, %edx
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_any_asn1_element@PLT
	.loc 1 203 8 discriminator 1
	testl	%eax, %eax
	jne	.L47
	.loc 1 204 14
	movl	$0, %eax
	jmp	.L50
.L47:
	.loc 1 206 18
	movq	-88(%rbp), %rax
	movq	$0, (%rax)
	.loc 1 207 12
	movl	$1, %eax
	jmp	.L50
.L46:
	.loc 1 211 8
	movq	-72(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, %rdx
	.loc 1 211 8 is_stmt 0 discriminator 1
	leaq	-48(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_init@PLT
	.loc 1 211 6 is_stmt 1 discriminator 2
	testl	%eax, %eax
	je	.L48
	.loc 1 212 8
	leaq	-48(%rbp), %rsi
	movq	-72(%rbp), %rax
	movl	$0, %r8d
	movl	$0, %ecx
	movl	$0, %edx
	movq	%rax, %rdi
	call	cbs_convert_ber
	.loc 1 211 36 discriminator 1
	testl	%eax, %eax
	je	.L48
	.loc 1 213 8
	leaq	-64(%rbp), %rdx
	movq	-88(%rbp), %rcx
	leaq	-48(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_finish@PLT
	.loc 1 212 43
	testl	%eax, %eax
	jne	.L49
.L48:
	.loc 1 214 5
	leaq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_cleanup@PLT
	.loc 1 215 12
	movl	$0, %eax
	jmp	.L50
.L49:
	.loc 1 218 3
	movq	-64(%rbp), %rdx
	movq	-88(%rbp), %rax
	movq	(%rax), %rcx
	movq	-80(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_init@PLT
	.loc 1 219 10
	movl	$1, %eax
.L50:
	.loc 1 220 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE4:
	.size	aws_lc_0_38_0_CBS_asn1_ber_to_der, .-aws_lc_0_38_0_CBS_asn1_ber_to_der
	.section	.rodata
	.align 8
.LC2:
	.string	"!(outer_tag & CBS_ASN1_CONSTRUCTED)"
	.align 8
.LC3:
	.string	"!(inner_tag & CBS_ASN1_CONSTRUCTED)"
.LC4:
	.string	"is_string_type(inner_tag)"
	.section	.text.aws_lc_0_38_0_CBS_get_asn1_implicit_string,"ax",@progbits
	.globl	aws_lc_0_38_0_CBS_get_asn1_implicit_string
	.type	aws_lc_0_38_0_CBS_get_asn1_implicit_string, @function
aws_lc_0_38_0_CBS_get_asn1_implicit_string:
.LFB5:
	.loc 1 224 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%rbx
	subq	$136, %rsp
	.cfi_offset 3, -24
	movq	%rdi, -120(%rbp)
	movq	%rsi, -128(%rbp)
	movq	%rdx, -136(%rbp)
	movl	%ecx, -140(%rbp)
	movl	%r8d, -144(%rbp)
	.loc 1 225 3
	movl	-140(%rbp), %eax
	andl	$536870912, %eax
	testl	%eax, %eax
	je	.L52
	.loc 1 225 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$225, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC2(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L52:
	.loc 1 226 3 is_stmt 1
	movl	-144(%rbp), %eax
	andl	$536870912, %eax
	testl	%eax, %eax
	je	.L53
	.loc 1 226 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$226, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC3(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L53:
	.loc 1 227 3 is_stmt 1
	movl	-144(%rbp), %eax
	movl	%eax, %edi
	call	is_string_type
	.loc 1 227 3 is_stmt 0 discriminator 1
	testl	%eax, %eax
	jne	.L54
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$227, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC4(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L54:
	.loc 1 229 7 is_stmt 1
	movl	-140(%rbp), %edx
	movq	-120(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_peek_asn1_tag@PLT
	.loc 1 229 6 discriminator 1
	testl	%eax, %eax
	je	.L55
	.loc 1 231 18
	movq	-136(%rbp), %rax
	movq	$0, (%rax)
	.loc 1 232 12
	movl	-140(%rbp), %edx
	movq	-128(%rbp), %rcx
	movq	-120(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1@PLT
	jmp	.L65
.L55:
	.loc 1 240 8
	movq	-120(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, %rdx
	.loc 1 240 8 is_stmt 0 discriminator 1
	leaq	-64(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_init@PLT
	.loc 1 240 6 is_stmt 1 discriminator 2
	testl	%eax, %eax
	je	.L66
	.loc 1 241 8
	movl	-140(%rbp), %eax
	orl	$536870912, %eax
	movl	%eax, %edx
	leaq	-80(%rbp), %rcx
	movq	-120(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1@PLT
	.loc 1 240 39 discriminator 1
	testl	%eax, %eax
	je	.L66
	.loc 1 245 9
	jmp	.L60
.L63:
.LBB5:
	.loc 1 247 10
	movl	-144(%rbp), %edx
	leaq	-112(%rbp), %rcx
	leaq	-80(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_get_asn1@PLT
	.loc 1 247 8 discriminator 1
	testl	%eax, %eax
	je	.L67
	.loc 1 248 10
	leaq	-112(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	movq	%rax, %rbx
	.loc 1 248 10 is_stmt 0 discriminator 1
	leaq	-112(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_data@PLT
	movq	%rax, %rcx
	.loc 1 248 10 discriminator 2
	leaq	-64(%rbp), %rax
	movq	%rbx, %rdx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_add_bytes@PLT
	.loc 1 247 50 is_stmt 1 discriminator 1
	testl	%eax, %eax
	je	.L67
.L60:
.LBE5:
	.loc 1 245 10
	leaq	-80(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_len@PLT
	.loc 1 245 26 discriminator 1
	testq	%rax, %rax
	jne	.L63
	.loc 1 255 8
	leaq	-96(%rbp), %rdx
	leaq	-88(%rbp), %rcx
	leaq	-64(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_finish@PLT
	.loc 1 255 6 discriminator 1
	testl	%eax, %eax
	je	.L68
	.loc 1 259 3
	movq	-96(%rbp), %rdx
	movq	-88(%rbp), %rcx
	movq	-128(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBS_init@PLT
	.loc 1 260 16
	movq	-88(%rbp), %rdx
	movq	-136(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 261 10
	movl	$1, %eax
	jmp	.L65
.L66:
	.loc 1 242 5
	nop
	jmp	.L59
.L67:
.LBB6:
	.loc 1 249 7 discriminator 1
	nop
	jmp	.L59
.L68:
.LBE6:
	.loc 1 256 5
	nop
.L59:
	.loc 1 264 3
	leaq	-64(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_cleanup@PLT
	.loc 1 265 10
	movl	$0, %eax
.L65:
	.loc 1 266 1
	movq	-8(%rbp), %rbx
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE5:
	.size	aws_lc_0_38_0_CBS_get_asn1_implicit_string, .-aws_lc_0_38_0_CBS_get_asn1_implicit_string
	.section	.rodata.__PRETTY_FUNCTION__.1,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.1, @object
	.size	__PRETTY_FUNCTION__.1, 16
__PRETTY_FUNCTION__.1:
	.string	"cbs_convert_ber"
	.section	.rodata.__PRETTY_FUNCTION__.0,"a"
	.align 32
	.type	__PRETTY_FUNCTION__.0, @object
	.size	__PRETTY_FUNCTION__.0, 43
__PRETTY_FUNCTION__.0:
	.string	"aws_lc_0_38_0_CBS_get_asn1_implicit_string"
	.text
.Letext0:
	.file 2 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 3 "/usr/include/bits/types.h"
	.file 4 "/usr/include/bits/stdint-uintn.h"
	.file 5 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.file 6 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/bytestring.h"
	.file 7 "/usr/include/assert.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x7fe
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF70
	.byte	0xc
	.long	.LASF71
	.long	.LASF72
	.long	.Ldebug_ranges0+0x30
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
	.byte	0x2a
	.byte	0x16
	.long	0x66
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF12
	.uleb128 0x5
	.long	0x93
	.uleb128 0x3
	.long	.LASF13
	.byte	0x4
	.byte	0x18
	.byte	0x13
	.long	0x74
	.uleb128 0x5
	.long	0x9f
	.uleb128 0x3
	.long	.LASF14
	.byte	0x4
	.byte	0x1a
	.byte	0x14
	.long	0x87
	.uleb128 0x5
	.long	0xb0
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF15
	.uleb128 0x6
	.long	.LASF16
	.byte	0x5
	.value	0x158
	.byte	0x12
	.long	0xb0
	.uleb128 0x7
	.string	"CBB"
	.byte	0x5
	.value	0x194
	.byte	0x17
	.long	0xe2
	.uleb128 0x8
	.long	.LASF19
	.byte	0x30
	.byte	0x6
	.value	0x1be
	.byte	0x8
	.long	0x119
	.uleb128 0x9
	.long	.LASF17
	.byte	0x6
	.value	0x1c0
	.byte	0x8
	.long	0x235
	.byte	0
	.uleb128 0x9
	.long	.LASF18
	.byte	0x6
	.value	0x1c3
	.byte	0x8
	.long	0x93
	.byte	0x8
	.uleb128 0xa
	.string	"u"
	.byte	0x6
	.value	0x1c7
	.byte	0x5
	.long	0x210
	.byte	0x10
	.byte	0
	.uleb128 0x7
	.string	"CBS"
	.byte	0x5
	.value	0x195
	.byte	0x17
	.long	0x12b
	.uleb128 0x5
	.long	0x119
	.uleb128 0xb
	.long	.LASF20
	.byte	0x10
	.byte	0x6
	.byte	0x28
	.byte	0x8
	.long	0x153
	.uleb128 0xc
	.long	.LASF21
	.byte	0x6
	.byte	0x29
	.byte	0x12
	.long	0x159
	.byte	0
	.uleb128 0xd
	.string	"len"
	.byte	0x6
	.byte	0x2a
	.byte	0xa
	.long	0x30
	.byte	0x8
	.byte	0
	.uleb128 0xe
	.byte	0x8
	.long	0x9a
	.uleb128 0xe
	.byte	0x8
	.long	0xab
	.uleb128 0x8
	.long	.LASF22
	.byte	0x20
	.byte	0x6
	.value	0x1a4
	.byte	0x8
	.long	0x1ba
	.uleb128 0xa
	.string	"buf"
	.byte	0x6
	.value	0x1a5
	.byte	0xc
	.long	0x1ba
	.byte	0
	.uleb128 0xa
	.string	"len"
	.byte	0x6
	.value	0x1a7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0xa
	.string	"cap"
	.byte	0x6
	.value	0x1a9
	.byte	0xa
	.long	0x30
	.byte	0x10
	.uleb128 0xf
	.long	.LASF23
	.byte	0x6
	.value	0x1ac
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x1f
	.byte	0x18
	.uleb128 0xf
	.long	.LASF24
	.byte	0x6
	.value	0x1af
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x1e
	.byte	0x18
	.byte	0
	.uleb128 0xe
	.byte	0x8
	.long	0x9f
	.uleb128 0x8
	.long	.LASF25
	.byte	0x18
	.byte	0x6
	.value	0x1b2
	.byte	0x8
	.long	0x20a
	.uleb128 0x9
	.long	.LASF26
	.byte	0x6
	.value	0x1b4
	.byte	0x19
	.long	0x20a
	.byte	0
	.uleb128 0x9
	.long	.LASF27
	.byte	0x6
	.value	0x1b7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0x9
	.long	.LASF28
	.byte	0x6
	.value	0x1ba
	.byte	0xb
	.long	0x9f
	.byte	0x10
	.uleb128 0xf
	.long	.LASF29
	.byte	0x6
	.value	0x1bb
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x17
	.byte	0x10
	.byte	0
	.uleb128 0xe
	.byte	0x8
	.long	0x15f
	.uleb128 0x10
	.byte	0x20
	.byte	0x6
	.value	0x1c4
	.byte	0x3
	.long	0x235
	.uleb128 0x11
	.long	.LASF26
	.byte	0x6
	.value	0x1c5
	.byte	0x1a
	.long	0x15f
	.uleb128 0x11
	.long	.LASF17
	.byte	0x6
	.value	0x1c6
	.byte	0x19
	.long	0x1c0
	.byte	0
	.uleb128 0xe
	.byte	0x8
	.long	0xd5
	.uleb128 0x12
	.long	.LASF48
	.byte	0x1
	.byte	0x18
	.byte	0x17
	.long	0xbc
	.uleb128 0x9
	.byte	0x3
	.quad	kMaxDepth
	.uleb128 0x13
	.long	.LASF30
	.byte	0x6
	.byte	0xf4
	.byte	0x14
	.long	0x43
	.long	0x271
	.uleb128 0x14
	.long	0x271
	.uleb128 0x14
	.long	0x271
	.uleb128 0x14
	.long	0xc8
	.byte	0
	.uleb128 0xe
	.byte	0x8
	.long	0x119
	.uleb128 0x15
	.long	.LASF31
	.byte	0x6
	.value	0x100
	.byte	0x14
	.long	0x43
	.long	0x293
	.uleb128 0x14
	.long	0x293
	.uleb128 0x14
	.long	0xc8
	.byte	0
	.uleb128 0xe
	.byte	0x8
	.long	0x126
	.uleb128 0x16
	.long	.LASF32
	.byte	0x6
	.byte	0x3d
	.byte	0x15
	.long	0x2b5
	.uleb128 0x14
	.long	0x271
	.uleb128 0x14
	.long	0x159
	.uleb128 0x14
	.long	0x30
	.byte	0
	.uleb128 0x17
	.long	.LASF33
	.byte	0x6
	.value	0x1e2
	.byte	0x15
	.long	0x2c8
	.uleb128 0x14
	.long	0x235
	.byte	0
	.uleb128 0x15
	.long	.LASF34
	.byte	0x6
	.value	0x1ec
	.byte	0x14
	.long	0x43
	.long	0x2e9
	.uleb128 0x14
	.long	0x235
	.uleb128 0x14
	.long	0x2e9
	.uleb128 0x14
	.long	0x2ef
	.byte	0
	.uleb128 0xe
	.byte	0x8
	.long	0x1ba
	.uleb128 0xe
	.byte	0x8
	.long	0x30
	.uleb128 0x15
	.long	.LASF35
	.byte	0x6
	.value	0x1d3
	.byte	0x14
	.long	0x43
	.long	0x311
	.uleb128 0x14
	.long	0x235
	.uleb128 0x14
	.long	0x30
	.byte	0
	.uleb128 0x15
	.long	.LASF36
	.byte	0x6
	.value	0x10c
	.byte	0x14
	.long	0x43
	.long	0x337
	.uleb128 0x14
	.long	0x271
	.uleb128 0x14
	.long	0x271
	.uleb128 0x14
	.long	0x337
	.uleb128 0x14
	.long	0x2ef
	.byte	0
	.uleb128 0xe
	.byte	0x8
	.long	0xc8
	.uleb128 0x15
	.long	.LASF37
	.byte	0x6
	.value	0x219
	.byte	0x14
	.long	0x43
	.long	0x35e
	.uleb128 0x14
	.long	0x235
	.uleb128 0x14
	.long	0x159
	.uleb128 0x14
	.long	0x30
	.byte	0
	.uleb128 0x15
	.long	.LASF38
	.byte	0x6
	.value	0x1f3
	.byte	0x14
	.long	0x43
	.long	0x375
	.uleb128 0x14
	.long	0x235
	.byte	0
	.uleb128 0x15
	.long	.LASF39
	.byte	0x6
	.value	0x215
	.byte	0x14
	.long	0x43
	.long	0x396
	.uleb128 0x14
	.long	0x235
	.uleb128 0x14
	.long	0x235
	.uleb128 0x14
	.long	0xc8
	.byte	0
	.uleb128 0x18
	.long	.LASF40
	.byte	0x7
	.byte	0x43
	.byte	0xd
	.long	0x3b7
	.uleb128 0x14
	.long	0x153
	.uleb128 0x14
	.long	0x153
	.uleb128 0x14
	.long	0x66
	.uleb128 0x14
	.long	0x153
	.byte	0
	.uleb128 0x13
	.long	.LASF41
	.byte	0x6
	.byte	0x44
	.byte	0x1f
	.long	0x159
	.long	0x3cd
	.uleb128 0x14
	.long	0x293
	.byte	0
	.uleb128 0x13
	.long	.LASF42
	.byte	0x6
	.byte	0x47
	.byte	0x17
	.long	0x30
	.long	0x3e3
	.uleb128 0x14
	.long	0x293
	.byte	0
	.uleb128 0x13
	.long	.LASF43
	.byte	0x6
	.byte	0x41
	.byte	0x14
	.long	0x43
	.long	0x3fe
	.uleb128 0x14
	.long	0x271
	.uleb128 0x14
	.long	0x30
	.byte	0
	.uleb128 0x15
	.long	.LASF44
	.byte	0x6
	.value	0x11c
	.byte	0x14
	.long	0x43
	.long	0x42e
	.uleb128 0x14
	.long	0x271
	.uleb128 0x14
	.long	0x271
	.uleb128 0x14
	.long	0x337
	.uleb128 0x14
	.long	0x2ef
	.uleb128 0x14
	.long	0x42e
	.uleb128 0x14
	.long	0x42e
	.byte	0
	.uleb128 0xe
	.byte	0x8
	.long	0x43
	.uleb128 0x19
	.long	.LASF51
	.byte	0x1
	.byte	0xde
	.byte	0x5
	.long	0x43
	.quad	.LFB5
	.quad	.LFE5-.LFB5
	.uleb128 0x1
	.byte	0x9c
	.long	0x520
	.uleb128 0x1a
	.string	"in"
	.byte	0x1
	.byte	0xde
	.byte	0x27
	.long	0x271
	.uleb128 0x3
	.byte	0x91
	.sleb128 -136
	.uleb128 0x1a
	.string	"out"
	.byte	0x1
	.byte	0xde
	.byte	0x30
	.long	0x271
	.uleb128 0x3
	.byte	0x91
	.sleb128 -144
	.uleb128 0x1b
	.long	.LASF45
	.byte	0x1
	.byte	0xde
	.byte	0x3f
	.long	0x2e9
	.uleb128 0x3
	.byte	0x91
	.sleb128 -152
	.uleb128 0x1b
	.long	.LASF46
	.byte	0x1
	.byte	0xdf
	.byte	0x2f
	.long	0xc8
	.uleb128 0x3
	.byte	0x91
	.sleb128 -156
	.uleb128 0x1b
	.long	.LASF47
	.byte	0x1
	.byte	0xe0
	.byte	0x2f
	.long	0xc8
	.uleb128 0x3
	.byte	0x91
	.sleb128 -160
	.uleb128 0x1c
	.long	.LASF57
	.long	0x530
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.0
	.uleb128 0x12
	.long	.LASF49
	.byte	0x1
	.byte	0xee
	.byte	0x7
	.long	0xd5
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x12
	.long	.LASF17
	.byte	0x1
	.byte	0xef
	.byte	0x7
	.long	0x119
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x1d
	.string	"err"
	.byte	0x1
	.value	0x107
	.byte	0x1
	.quad	.L59
	.uleb128 0x12
	.long	.LASF21
	.byte	0x1
	.byte	0xfd
	.byte	0xc
	.long	0x1ba
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x1e
	.string	"len"
	.byte	0x1
	.byte	0xfe
	.byte	0xa
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x1f
	.long	.Ldebug_ranges0+0
	.uleb128 0x12
	.long	.LASF50
	.byte	0x1
	.byte	0xf6
	.byte	0x9
	.long	0x119
	.uleb128 0x3
	.byte	0x91
	.sleb128 -128
	.byte	0
	.byte	0
	.uleb128 0x20
	.long	0x9a
	.long	0x530
	.uleb128 0x21
	.long	0x3c
	.byte	0x2a
	.byte	0
	.uleb128 0x5
	.long	0x520
	.uleb128 0x19
	.long	.LASF52
	.byte	0x1
	.byte	0xc0
	.byte	0x5
	.long	0x43
	.quad	.LFB4
	.quad	.LFE4-.LFB4
	.uleb128 0x1
	.byte	0x9c
	.long	0x5b6
	.uleb128 0x1a
	.string	"in"
	.byte	0x1
	.byte	0xc0
	.byte	0x1e
	.long	0x271
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x1a
	.string	"out"
	.byte	0x1
	.byte	0xc0
	.byte	0x27
	.long	0x271
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x1b
	.long	.LASF45
	.byte	0x1
	.byte	0xc0
	.byte	0x36
	.long	0x2e9
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x1e
	.string	"cbb"
	.byte	0x1
	.byte	0xc1
	.byte	0x7
	.long	0xd5
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x12
	.long	.LASF53
	.byte	0x1
	.byte	0xc5
	.byte	0x7
	.long	0x43
	.uleb128 0x3
	.byte	0x91
	.sleb128 -68
	.uleb128 0x1e
	.string	"len"
	.byte	0x1
	.byte	0xd2
	.byte	0xa
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.byte	0
	.uleb128 0x22
	.long	.LASF65
	.byte	0x1
	.byte	0x70
	.byte	0xc
	.long	0x43
	.quad	.LFB3
	.quad	.LFE3-.LFB3
	.uleb128 0x1
	.byte	0x9c
	.long	0x6dc
	.uleb128 0x1a
	.string	"in"
	.byte	0x1
	.byte	0x70
	.byte	0x21
	.long	0x271
	.uleb128 0x3
	.byte	0x91
	.sleb128 -168
	.uleb128 0x1a
	.string	"out"
	.byte	0x1
	.byte	0x70
	.byte	0x2a
	.long	0x235
	.uleb128 0x3
	.byte	0x91
	.sleb128 -176
	.uleb128 0x1b
	.long	.LASF54
	.byte	0x1
	.byte	0x70
	.byte	0x3c
	.long	0xc8
	.uleb128 0x3
	.byte	0x91
	.sleb128 -180
	.uleb128 0x1b
	.long	.LASF55
	.byte	0x1
	.byte	0x71
	.byte	0x20
	.long	0x43
	.uleb128 0x3
	.byte	0x91
	.sleb128 -184
	.uleb128 0x1b
	.long	.LASF56
	.byte	0x1
	.byte	0x71
	.byte	0x3a
	.long	0xb0
	.uleb128 0x3
	.byte	0x91
	.sleb128 -188
	.uleb128 0x1c
	.long	.LASF57
	.long	0x6ec
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.1
	.uleb128 0x23
	.quad	.LBB3
	.quad	.LBE3-.LBB3
	.uleb128 0x12
	.long	.LASF58
	.byte	0x1
	.byte	0x7d
	.byte	0x9
	.long	0x119
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x1e
	.string	"tag"
	.byte	0x1
	.byte	0x7e
	.byte	0x12
	.long	0xc8
	.uleb128 0x3
	.byte	0x91
	.sleb128 -84
	.uleb128 0x12
	.long	.LASF59
	.byte	0x1
	.byte	0x7e
	.byte	0x17
	.long	0xc8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.uleb128 0x12
	.long	.LASF60
	.byte	0x1
	.byte	0x7f
	.byte	0xc
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x12
	.long	.LASF61
	.byte	0x1
	.byte	0x80
	.byte	0x9
	.long	0x43
	.uleb128 0x3
	.byte	0x91
	.sleb128 -100
	.uleb128 0x12
	.long	.LASF62
	.byte	0x1
	.byte	0x81
	.byte	0xa
	.long	0x235
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x12
	.long	.LASF63
	.byte	0x1
	.byte	0x81
	.byte	0x18
	.long	0xd5
	.uleb128 0x3
	.byte	0x91
	.sleb128 -160
	.uleb128 0x23
	.quad	.LBB4
	.quad	.LBE4-.LBB4
	.uleb128 0x12
	.long	.LASF64
	.byte	0x1
	.byte	0x90
	.byte	0x14
	.long	0xc8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -52
	.byte	0
	.byte	0
	.byte	0
	.uleb128 0x20
	.long	0x9a
	.long	0x6ec
	.uleb128 0x21
	.long	0x3c
	.byte	0xf
	.byte	0
	.uleb128 0x5
	.long	0x6dc
	.uleb128 0x22
	.long	.LASF66
	.byte	0x1
	.byte	0x61
	.byte	0xc
	.long	0x43
	.quad	.LFB2
	.quad	.LFE2-.LFB2
	.uleb128 0x1
	.byte	0x9c
	.long	0x723
	.uleb128 0x1a
	.string	"cbs"
	.byte	0x1
	.byte	0x61
	.byte	0x1d
	.long	0x271
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x22
	.long	.LASF67
	.byte	0x1
	.byte	0x37
	.byte	0xc
	.long	0x43
	.quad	.LFB1
	.quad	.LFE1-.LFB1
	.uleb128 0x1
	.byte	0x9c
	.long	0x7d3
	.uleb128 0x1b
	.long	.LASF68
	.byte	0x1
	.byte	0x37
	.byte	0x24
	.long	0x293
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x1b
	.long	.LASF69
	.byte	0x1
	.byte	0x37
	.byte	0x32
	.long	0x42e
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x1b
	.long	.LASF56
	.byte	0x1
	.byte	0x37
	.byte	0x46
	.long	0xb0
	.uleb128 0x3
	.byte	0x91
	.sleb128 -100
	.uleb128 0x1e
	.string	"in"
	.byte	0x1
	.byte	0x3c
	.byte	0x7
	.long	0x119
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x23
	.quad	.LBB2
	.quad	.LBE2-.LBB2
	.uleb128 0x12
	.long	.LASF58
	.byte	0x1
	.byte	0x40
	.byte	0x9
	.long	0x119
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x1e
	.string	"tag"
	.byte	0x1
	.byte	0x41
	.byte	0x12
	.long	0xc8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -52
	.uleb128 0x12
	.long	.LASF60
	.byte	0x1
	.byte	0x42
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x12
	.long	.LASF61
	.byte	0x1
	.byte	0x43
	.byte	0x9
	.long	0x43
	.uleb128 0x3
	.byte	0x91
	.sleb128 -68
	.byte	0
	.byte	0
	.uleb128 0x24
	.long	.LASF73
	.byte	0x1
	.byte	0x1c
	.byte	0xc
	.long	0x43
	.quad	.LFB0
	.quad	.LFE0-.LFB0
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x1a
	.string	"tag"
	.byte	0x1
	.byte	0x1c
	.byte	0x28
	.long	0xc8
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
	.uleb128 0x7
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
	.uleb128 0x8
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
	.uleb128 0x9
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
	.uleb128 0xa
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
	.uleb128 0xb
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
	.uleb128 0xe
	.uleb128 0xf
	.byte	0
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x49
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
	.uleb128 0x10
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
	.uleb128 0x11
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
	.uleb128 0x12
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
	.uleb128 0x5
	.byte	0
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
	.uleb128 0x1a
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
	.uleb128 0x1b
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
	.uleb128 0x1c
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
	.uleb128 0x1d
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
	.uleb128 0x1e
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
	.uleb128 0x1f
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x55
	.uleb128 0x17
	.byte	0
	.byte	0
	.uleb128 0x20
	.uleb128 0x1
	.byte	0x1
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x21
	.uleb128 0x21
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2f
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x22
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
	.uleb128 0x23
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.byte	0
	.byte	0
	.uleb128 0x24
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
	.long	0x7c
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
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LBB5
	.quad	.LBE5
	.quad	.LBB6
	.quad	.LBE6
	.quad	0
	.quad	0
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
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF28:
	.string	"pending_len_len"
.LASF66:
	.string	"cbs_get_eoc"
.LASF30:
	.string	"aws_lc_0_38_0_CBS_get_asn1"
.LASF19:
	.string	"cbb_st"
.LASF23:
	.string	"can_resize"
.LASF65:
	.string	"cbs_convert_ber"
.LASF8:
	.string	"size_t"
.LASF64:
	.string	"out_tag"
.LASF57:
	.string	"__PRETTY_FUNCTION__"
.LASF43:
	.string	"aws_lc_0_38_0_CBS_skip"
.LASF2:
	.string	"long long int"
.LASF11:
	.string	"__uint32_t"
.LASF25:
	.string	"cbb_child_st"
.LASF48:
	.string	"kMaxDepth"
.LASF36:
	.string	"aws_lc_0_38_0_CBS_get_any_asn1_element"
.LASF37:
	.string	"aws_lc_0_38_0_CBB_add_bytes"
.LASF58:
	.string	"contents"
.LASF32:
	.string	"aws_lc_0_38_0_CBS_init"
.LASF13:
	.string	"uint8_t"
.LASF18:
	.string	"is_child"
.LASF41:
	.string	"aws_lc_0_38_0_CBS_data"
.LASF10:
	.string	"short int"
.LASF69:
	.string	"ber_found"
.LASF63:
	.string	"out_contents_storage"
.LASF51:
	.string	"aws_lc_0_38_0_CBS_get_asn1_implicit_string"
.LASF71:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/ber.c"
.LASF61:
	.string	"indefinite"
.LASF0:
	.string	"long int"
.LASF27:
	.string	"offset"
.LASF49:
	.string	"result"
.LASF9:
	.string	"__uint8_t"
.LASF34:
	.string	"aws_lc_0_38_0_CBB_finish"
.LASF55:
	.string	"looking_for_eoc"
.LASF70:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF3:
	.string	"long double"
.LASF29:
	.string	"pending_is_asn1"
.LASF20:
	.string	"cbs_st"
.LASF4:
	.string	"unsigned char"
.LASF16:
	.string	"CBS_ASN1_TAG"
.LASF7:
	.string	"signed char"
.LASF15:
	.string	"long long unsigned int"
.LASF60:
	.string	"header_len"
.LASF14:
	.string	"uint32_t"
.LASF73:
	.string	"is_string_type"
.LASF6:
	.string	"unsigned int"
.LASF38:
	.string	"aws_lc_0_38_0_CBB_flush"
.LASF31:
	.string	"aws_lc_0_38_0_CBS_peek_asn1_tag"
.LASF22:
	.string	"cbb_buffer_st"
.LASF54:
	.string	"string_tag"
.LASF5:
	.string	"short unsigned int"
.LASF12:
	.string	"char"
.LASF42:
	.string	"aws_lc_0_38_0_CBS_len"
.LASF47:
	.string	"inner_tag"
.LASF33:
	.string	"aws_lc_0_38_0_CBB_cleanup"
.LASF59:
	.string	"child_string_tag"
.LASF56:
	.string	"depth"
.LASF21:
	.string	"data"
.LASF53:
	.string	"conversion_needed"
.LASF39:
	.string	"aws_lc_0_38_0_CBB_add_asn1"
.LASF1:
	.string	"long unsigned int"
.LASF17:
	.string	"child"
.LASF62:
	.string	"out_contents"
.LASF35:
	.string	"aws_lc_0_38_0_CBB_init"
.LASF44:
	.string	"aws_lc_0_38_0_CBS_get_any_ber_asn1_element"
.LASF72:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF24:
	.string	"error"
.LASF46:
	.string	"outer_tag"
.LASF52:
	.string	"aws_lc_0_38_0_CBS_asn1_ber_to_der"
.LASF40:
	.string	"__assert_fail"
.LASF26:
	.string	"base"
.LASF68:
	.string	"orig_in"
.LASF45:
	.string	"out_storage"
.LASF50:
	.string	"chunk"
.LASF67:
	.string	"cbs_find_ber"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
