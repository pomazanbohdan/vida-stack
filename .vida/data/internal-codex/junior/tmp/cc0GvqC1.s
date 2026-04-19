	.file	"asn1_compat.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/asn1_compat.c"
	.section	.text.OPENSSL_memcpy,"ax",@progbits
	.type	OPENSSL_memcpy, @function
OPENSSL_memcpy:
.LFB94:
	.file 2 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/../internal.h"
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
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/asn1_compat.c"
.LC1:
	.string	"!cbb->is_child"
.LC2:
	.string	"cbb->u.base.can_resize"
	.section	.text.aws_lc_0_38_0_CBB_finish_i2d,"ax",@progbits
	.globl	aws_lc_0_38_0_CBB_finish_i2d
	.type	aws_lc_0_38_0_CBB_finish_i2d, @function
aws_lc_0_38_0_CBB_finish_i2d:
.LFB128:
	.loc 1 28 46
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 29 3
	movq	-24(%rbp), %rax
	movzbl	8(%rax), %eax
	testb	%al, %al
	je	.L5
	.loc 1 29 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$29, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC1(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L5:
	.loc 1 30 3 is_stmt 1
	movq	-24(%rbp), %rax
	movzbl	40(%rax), %eax
	andl	$1, %eax
	testb	%al, %al
	jne	.L6
	.loc 1 30 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$30, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC2(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L6:
	.loc 1 34 8 is_stmt 1
	leaq	-16(%rbp), %rdx
	leaq	-8(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_finish@PLT
	.loc 1 34 6 discriminator 1
	testl	%eax, %eax
	jne	.L7
	.loc 1 35 5
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CBB_cleanup@PLT
	.loc 1 36 12
	movl	$-1, %eax
	jmp	.L12
.L7:
	.loc 1 38 15
	movq	-16(%rbp), %rdx
	.loc 1 38 6
	movl	$2147483648, %eax
	cmpq	%rax, %rdx
	jb	.L9
	.loc 1 39 5
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_free@PLT
	.loc 1 40 12
	movl	$-1, %eax
	jmp	.L12
.L9:
	.loc 1 42 6
	cmpq	$0, -32(%rbp)
	je	.L10
	.loc 1 43 9
	movq	-32(%rbp), %rax
	movq	(%rax), %rax
	.loc 1 43 8
	testq	%rax, %rax
	jne	.L11
	.loc 1 44 13
	movq	-8(%rbp), %rdx
	movq	-32(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 45 11
	movq	$0, -8(%rbp)
	jmp	.L10
.L11:
	.loc 1 47 7
	movq	-16(%rbp), %rdx
	movq	-8(%rbp), %rcx
	movq	-32(%rbp), %rax
	movq	(%rax), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 48 7
	movq	-32(%rbp), %rax
	movq	(%rax), %rdx
	.loc 1 48 13
	movq	-16(%rbp), %rax
	addq	%rax, %rdx
	movq	-32(%rbp), %rax
	movq	%rdx, (%rax)
.L10:
	.loc 1 51 3
	movq	-8(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_free@PLT
	.loc 1 52 10
	movq	-16(%rbp), %rax
.L12:
	.loc 1 53 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE128:
	.size	aws_lc_0_38_0_CBB_finish_i2d, .-aws_lc_0_38_0_CBB_finish_i2d
	.section	.rodata.__PRETTY_FUNCTION__.0,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.0, @object
	.size	__PRETTY_FUNCTION__.0, 29
__PRETTY_FUNCTION__.0:
	.string	"aws_lc_0_38_0_CBB_finish_i2d"
	.text
.Letext0:
	.file 3 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 4 "/usr/include/bits/types.h"
	.file 5 "/usr/include/bits/stdint-uintn.h"
	.file 6 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/bytestring.h"
	.file 7 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/mem.h"
	.file 8 "/usr/include/string.h"
	.file 9 "/usr/include/assert.h"
	.file 10 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x356
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF36
	.byte	0xc
	.long	.LASF37
	.long	.LASF38
	.long	.Ldebug_ranges0+0
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
	.uleb128 0x5
	.byte	0x8
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF11
	.uleb128 0x6
	.long	0x89
	.uleb128 0x3
	.long	.LASF12
	.byte	0x5
	.byte	0x18
	.byte	0x13
	.long	0x74
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF13
	.uleb128 0x7
	.byte	0x8
	.long	0xae
	.uleb128 0x8
	.uleb128 0x9
	.string	"CBB"
	.byte	0xa
	.value	0x194
	.byte	0x17
	.long	0xbc
	.uleb128 0xa
	.long	.LASF16
	.byte	0x30
	.byte	0x6
	.value	0x1be
	.byte	0x8
	.long	0xf3
	.uleb128 0xb
	.long	.LASF14
	.byte	0x6
	.value	0x1c0
	.byte	0x8
	.long	0x1cf
	.byte	0
	.uleb128 0xb
	.long	.LASF15
	.byte	0x6
	.value	0x1c3
	.byte	0x8
	.long	0x89
	.byte	0x8
	.uleb128 0xc
	.string	"u"
	.byte	0x6
	.value	0x1c7
	.byte	0x5
	.long	0x1aa
	.byte	0x10
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x90
	.uleb128 0xa
	.long	.LASF17
	.byte	0x20
	.byte	0x6
	.value	0x1a4
	.byte	0x8
	.long	0x154
	.uleb128 0xc
	.string	"buf"
	.byte	0x6
	.value	0x1a5
	.byte	0xc
	.long	0x154
	.byte	0
	.uleb128 0xc
	.string	"len"
	.byte	0x6
	.value	0x1a7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0xc
	.string	"cap"
	.byte	0x6
	.value	0x1a9
	.byte	0xa
	.long	0x30
	.byte	0x10
	.uleb128 0xd
	.long	.LASF18
	.byte	0x6
	.value	0x1ac
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x1f
	.byte	0x18
	.uleb128 0xd
	.long	.LASF19
	.byte	0x6
	.value	0x1af
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x1e
	.byte	0x18
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x95
	.uleb128 0xa
	.long	.LASF20
	.byte	0x18
	.byte	0x6
	.value	0x1b2
	.byte	0x8
	.long	0x1a4
	.uleb128 0xb
	.long	.LASF21
	.byte	0x6
	.value	0x1b4
	.byte	0x19
	.long	0x1a4
	.byte	0
	.uleb128 0xb
	.long	.LASF22
	.byte	0x6
	.value	0x1b7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0xb
	.long	.LASF23
	.byte	0x6
	.value	0x1ba
	.byte	0xb
	.long	0x95
	.byte	0x10
	.uleb128 0xd
	.long	.LASF24
	.byte	0x6
	.value	0x1bb
	.byte	0xc
	.long	0x66
	.byte	0x4
	.byte	0x1
	.byte	0x17
	.byte	0x10
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xf9
	.uleb128 0xe
	.byte	0x20
	.byte	0x6
	.value	0x1c4
	.byte	0x3
	.long	0x1cf
	.uleb128 0xf
	.long	.LASF21
	.byte	0x6
	.value	0x1c5
	.byte	0x1a
	.long	0xf9
	.uleb128 0xf
	.long	.LASF14
	.byte	0x6
	.value	0x1c6
	.byte	0x19
	.long	0x15a
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xaf
	.uleb128 0x2
	.byte	0x8
	.byte	0x4
	.long	.LASF25
	.uleb128 0x2
	.byte	0x10
	.byte	0x5
	.long	.LASF26
	.uleb128 0x2
	.byte	0x10
	.byte	0x7
	.long	.LASF27
	.uleb128 0x2
	.byte	0x1
	.byte	0x2
	.long	.LASF28
	.uleb128 0x10
	.long	.LASF31
	.byte	0x8
	.byte	0x2b
	.byte	0xe
	.long	0x87
	.long	0x211
	.uleb128 0x11
	.long	0x87
	.uleb128 0x11
	.long	0xa8
	.uleb128 0x11
	.long	0x3c
	.byte	0
	.uleb128 0x12
	.long	.LASF29
	.byte	0x7
	.byte	0x69
	.byte	0x15
	.long	0x223
	.uleb128 0x11
	.long	0x87
	.byte	0
	.uleb128 0x13
	.long	.LASF30
	.byte	0x6
	.value	0x1e2
	.byte	0x15
	.long	0x236
	.uleb128 0x11
	.long	0x1cf
	.byte	0
	.uleb128 0x14
	.long	.LASF32
	.byte	0x6
	.value	0x1ec
	.byte	0x14
	.long	0x43
	.long	0x257
	.uleb128 0x11
	.long	0x1cf
	.uleb128 0x11
	.long	0x257
	.uleb128 0x11
	.long	0x25d
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x154
	.uleb128 0x7
	.byte	0x8
	.long	0x30
	.uleb128 0x15
	.long	.LASF33
	.byte	0x9
	.byte	0x43
	.byte	0xd
	.long	0x284
	.uleb128 0x11
	.long	0xf3
	.uleb128 0x11
	.long	0xf3
	.uleb128 0x11
	.long	0x66
	.uleb128 0x11
	.long	0xf3
	.byte	0
	.uleb128 0x16
	.long	.LASF39
	.byte	0x1
	.byte	0x1c
	.byte	0x5
	.long	0x43
	.quad	.LFB128
	.quad	.LFE128-.LFB128
	.uleb128 0x1
	.byte	0x9c
	.long	0x2f6
	.uleb128 0x17
	.string	"cbb"
	.byte	0x1
	.byte	0x1c
	.byte	0x19
	.long	0x1cf
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x18
	.long	.LASF34
	.byte	0x1
	.byte	0x1c
	.byte	0x28
	.long	0x257
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x19
	.long	.LASF40
	.long	0x306
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.0
	.uleb128 0x1a
	.string	"der"
	.byte	0x1
	.byte	0x20
	.byte	0xc
	.long	0x154
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1b
	.long	.LASF35
	.byte	0x1
	.byte	0x21
	.byte	0xa
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x1c
	.long	0x90
	.long	0x306
	.uleb128 0x1d
	.long	0x3c
	.byte	0x1c
	.byte	0
	.uleb128 0x6
	.long	0x2f6
	.uleb128 0x1e
	.long	.LASF41
	.byte	0x2
	.value	0x3bc
	.byte	0x15
	.long	0x87
	.quad	.LFB94
	.quad	.LFE94-.LFB94
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x1f
	.string	"dst"
	.byte	0x2
	.value	0x3bc
	.byte	0x2a
	.long	0x87
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1f
	.string	"src"
	.byte	0x2
	.value	0x3bc
	.byte	0x3b
	.long	0xa8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1f
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
	.uleb128 0x26
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
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
	.uleb128 0x5
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
	.uleb128 0x5
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
	.uleb128 0xe
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
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x3c
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x11
	.uleb128 0x5
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x12
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
	.uleb128 0x87
	.uleb128 0x19
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
	.uleb128 0x17
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
	.uleb128 0x18
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
	.uleb128 0x19
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
	.uleb128 0x1a
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
	.uleb128 0x1b
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
	.uleb128 0x1c
	.uleb128 0x1
	.byte	0x1
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x1d
	.uleb128 0x21
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2f
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x1e
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
	.uleb128 0x1f
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
	.byte	0
	.section	.debug_aranges,"",@progbits
	.long	0x3c
	.value	0x2
	.long	.Ldebug_info0
	.byte	0x8
	.byte	0
	.value	0
	.value	0
	.quad	.LFB94
	.quad	.LFE94-.LFB94
	.quad	.LFB128
	.quad	.LFE128-.LFB128
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LFB94
	.quad	.LFE94
	.quad	.LFB128
	.quad	.LFE128
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF27:
	.string	"__int128 unsigned"
.LASF8:
	.string	"size_t"
.LASF9:
	.string	"__uint8_t"
.LASF16:
	.string	"cbb_st"
.LASF17:
	.string	"cbb_buffer_st"
.LASF6:
	.string	"unsigned int"
.LASF32:
	.string	"aws_lc_0_38_0_CBB_finish"
.LASF38:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF41:
	.string	"OPENSSL_memcpy"
.LASF15:
	.string	"is_child"
.LASF40:
	.string	"__PRETTY_FUNCTION__"
.LASF14:
	.string	"child"
.LASF4:
	.string	"unsigned char"
.LASF23:
	.string	"pending_len_len"
.LASF1:
	.string	"long unsigned int"
.LASF20:
	.string	"cbb_child_st"
.LASF37:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/bytestring/asn1_compat.c"
.LASF26:
	.string	"__int128"
.LASF25:
	.string	"double"
.LASF18:
	.string	"can_resize"
.LASF35:
	.string	"der_len"
.LASF39:
	.string	"aws_lc_0_38_0_CBB_finish_i2d"
.LASF29:
	.string	"aws_lc_0_38_0_OPENSSL_free"
.LASF21:
	.string	"base"
.LASF33:
	.string	"__assert_fail"
.LASF11:
	.string	"char"
.LASF12:
	.string	"uint8_t"
.LASF34:
	.string	"outp"
.LASF36:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF19:
	.string	"error"
.LASF2:
	.string	"long long int"
.LASF31:
	.string	"memcpy"
.LASF22:
	.string	"offset"
.LASF10:
	.string	"short int"
.LASF0:
	.string	"long int"
.LASF30:
	.string	"aws_lc_0_38_0_CBB_cleanup"
.LASF3:
	.string	"long double"
.LASF7:
	.string	"signed char"
.LASF5:
	.string	"short unsigned int"
.LASF24:
	.string	"pending_is_asn1"
.LASF28:
	.string	"_Bool"
.LASF13:
	.string	"long long unsigned int"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
