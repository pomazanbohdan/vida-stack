	.file	"derive_key.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/derive_key.c"
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/derive_key.c"
.LC1:
	.string	"nkey <= EVP_MAX_KEY_LENGTH"
.LC2:
	.string	"niv <= EVP_MAX_IV_LENGTH"
	.section	.text.aws_lc_0_38_0_EVP_BytesToKey,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_BytesToKey
	.type	aws_lc_0_38_0_EVP_BytesToKey, @function
aws_lc_0_38_0_EVP_BytesToKey:
.LFB171:
	.loc 1 68 63
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$208, %rsp
	movq	%rdi, -168(%rbp)
	movq	%rsi, -176(%rbp)
	movq	%rdx, -184(%rbp)
	movq	%rcx, -192(%rbp)
	movq	%r8, -200(%rbp)
	movl	%r9d, -204(%rbp)
	.loc 1 71 12
	movl	$0, -4(%rbp)
	.loc 1 72 12
	movl	$0, -148(%rbp)
	.loc 1 73 7
	movl	$0, -12(%rbp)
	.loc 1 75 19
	movq	-168(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_CIPHER_key_length@PLT
	movl	%eax, -16(%rbp)
	.loc 1 76 18
	movq	-168(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_CIPHER_iv_length@PLT
	movl	%eax, -20(%rbp)
	.loc 1 78 3
	cmpl	$64, -16(%rbp)
	jbe	.L2
	.loc 1 78 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$78, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC1(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L2:
	.loc 1 79 3 is_stmt 1
	cmpl	$16, -20(%rbp)
	jbe	.L3
	.loc 1 79 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$79, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC2(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L3:
	.loc 1 81 6 is_stmt 1
	cmpq	$0, -192(%rbp)
	jne	.L4
	.loc 1 82 12
	movl	-16(%rbp), %eax
	jmp	.L25
.L4:
	.loc 1 85 3
	leaq	-80(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_MD_CTX_init@PLT
.L24:
	.loc 1 87 10
	movq	-176(%rbp), %rcx
	leaq	-80(%rbp), %rax
	movl	$0, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_DigestInit_ex@PLT
	.loc 1 87 8 discriminator 1
	testl	%eax, %eax
	je	.L27
	.loc 1 90 14
	movl	-4(%rbp), %eax
	leal	1(%rax), %edx
	movl	%edx, -4(%rbp)
	.loc 1 90 8
	testl	%eax, %eax
	je	.L8
	.loc 1 91 12
	movl	-148(%rbp), %eax
	movl	%eax, %edx
	leaq	-144(%rbp), %rcx
	leaq	-80(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_DigestUpdate@PLT
	.loc 1 91 10 discriminator 1
	testl	%eax, %eax
	je	.L28
.L8:
	.loc 1 95 10
	movq	-200(%rbp), %rdx
	movq	-192(%rbp), %rcx
	leaq	-80(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_DigestUpdate@PLT
	.loc 1 95 8 discriminator 1
	testl	%eax, %eax
	je	.L29
	.loc 1 98 8
	cmpq	$0, -184(%rbp)
	je	.L10
	.loc 1 99 12
	movq	-184(%rbp), %rcx
	leaq	-80(%rbp), %rax
	movl	$8, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_DigestUpdate@PLT
	.loc 1 99 10 discriminator 1
	testl	%eax, %eax
	je	.L30
.L10:
	.loc 1 103 10
	leaq	-148(%rbp), %rdx
	leaq	-144(%rbp), %rcx
	leaq	-80(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_DigestFinal_ex@PLT
	.loc 1 103 8 discriminator 1
	testl	%eax, %eax
	je	.L31
	.loc 1 107 12
	movl	$1, -8(%rbp)
	.loc 1 107 5
	jmp	.L12
.L15:
	.loc 1 108 12
	movq	-176(%rbp), %rcx
	leaq	-80(%rbp), %rax
	movl	$0, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_DigestInit_ex@PLT
	.loc 1 108 10 discriminator 1
	testl	%eax, %eax
	je	.L32
	.loc 1 109 12
	movl	-148(%rbp), %eax
	movl	%eax, %edx
	leaq	-144(%rbp), %rcx
	leaq	-80(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_DigestUpdate@PLT
	.loc 1 108 44 discriminator 1
	testl	%eax, %eax
	je	.L32
	.loc 1 110 12
	leaq	-148(%rbp), %rdx
	leaq	-144(%rbp), %rcx
	leaq	-80(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_DigestFinal_ex@PLT
	.loc 1 109 46
	testl	%eax, %eax
	je	.L32
	.loc 1 107 29 discriminator 2
	addl	$1, -8(%rbp)
.L12:
	.loc 1 107 19 discriminator 1
	movl	-8(%rbp), %eax
	cmpl	-204(%rbp), %eax
	jb	.L15
	.loc 1 115 7
	movl	$0, -8(%rbp)
	.loc 1 116 8
	cmpl	$0, -16(%rbp)
	je	.L16
.L18:
	.loc 1 118 12
	cmpl	$0, -16(%rbp)
	je	.L16
	.loc 1 118 28 discriminator 1
	movl	-148(%rbp), %eax
	.loc 1 118 23 discriminator 1
	cmpl	%eax, -8(%rbp)
	je	.L16
	.loc 1 121 12
	cmpq	$0, 16(%rbp)
	je	.L17
	.loc 1 122 16
	movq	16(%rbp), %rax
	leaq	1(%rax), %rdx
	movq	%rdx, 16(%rbp)
	.loc 1 122 28
	movl	-8(%rbp), %edx
	movzbl	-144(%rbp,%rdx), %edx
	.loc 1 122 20
	movb	%dl, (%rax)
.L17:
	.loc 1 124 13
	subl	$1, -16(%rbp)
	.loc 1 125 10
	addl	$1, -8(%rbp)
	.loc 1 118 12
	jmp	.L18
.L16:
	.loc 1 129 8
	cmpl	$0, -20(%rbp)
	je	.L19
	.loc 1 129 18 discriminator 1
	movl	-148(%rbp), %eax
	.loc 1 129 13 discriminator 1
	cmpl	%eax, -8(%rbp)
	je	.L19
.L21:
	.loc 1 131 12
	cmpl	$0, -20(%rbp)
	je	.L19
	.loc 1 131 27 discriminator 1
	movl	-148(%rbp), %eax
	.loc 1 131 22 discriminator 1
	cmpl	%eax, -8(%rbp)
	je	.L19
	.loc 1 134 12
	cmpq	$0, 24(%rbp)
	je	.L20
	.loc 1 135 15
	movq	24(%rbp), %rax
	leaq	1(%rax), %rdx
	movq	%rdx, 24(%rbp)
	.loc 1 135 27
	movl	-8(%rbp), %edx
	movzbl	-144(%rbp,%rdx), %edx
	.loc 1 135 19
	movb	%dl, (%rax)
.L20:
	.loc 1 137 12
	subl	$1, -20(%rbp)
	.loc 1 138 10
	addl	$1, -8(%rbp)
	.loc 1 131 12
	jmp	.L21
.L19:
	.loc 1 141 8
	cmpl	$0, -16(%rbp)
	jne	.L24
	.loc 1 141 19 discriminator 1
	cmpl	$0, -20(%rbp)
	je	.L33
	.loc 1 87 8
	jmp	.L24
.L33:
	.loc 1 142 7
	nop
	.loc 1 145 8
	movq	-168(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_CIPHER_key_length@PLT
	.loc 1 145 6 discriminator 1
	movl	%eax, -12(%rbp)
	jmp	.L7
.L27:
	.loc 1 88 7
	nop
	jmp	.L7
.L28:
	.loc 1 92 9
	nop
	jmp	.L7
.L29:
	.loc 1 96 7
	nop
	jmp	.L7
.L30:
	.loc 1 100 9
	nop
	jmp	.L7
.L31:
	.loc 1 104 7
	nop
	jmp	.L7
.L32:
	.loc 1 111 9
	nop
.L7:
	.loc 1 148 3
	leaq	-80(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_MD_CTX_cleanup@PLT
	.loc 1 149 3
	leaq	-144(%rbp), %rax
	movl	$64, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_cleanse@PLT
	.loc 1 150 10
	movl	-12(%rbp), %eax
.L25:
	.loc 1 151 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE171:
	.size	aws_lc_0_38_0_EVP_BytesToKey, .-aws_lc_0_38_0_EVP_BytesToKey
	.section	.rodata.__PRETTY_FUNCTION__.0,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.0, @object
	.size	__PRETTY_FUNCTION__.0, 29
__PRETTY_FUNCTION__.0:
	.string	"aws_lc_0_38_0_EVP_BytesToKey"
	.text
.Letext0:
	.file 2 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 3 "/usr/include/bits/types.h"
	.file 4 "/usr/include/bits/stdint-uintn.h"
	.file 5 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.file 6 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/digest.h"
	.file 7 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/mem.h"
	.file 8 "/usr/include/assert.h"
	.file 9 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/cipher.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x42a
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF48
	.byte	0xc
	.long	.LASF49
	.long	.LASF50
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
	.byte	0x4
	.byte	0x18
	.byte	0x13
	.long	0x74
	.uleb128 0x6
	.long	0x95
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF13
	.uleb128 0x7
	.byte	0x8
	.long	0xb3
	.uleb128 0x8
	.uleb128 0x9
	.long	.LASF14
	.byte	0x5
	.value	0x1a3
	.byte	0x1a
	.long	0xc1
	.uleb128 0xa
	.long	.LASF23
	.uleb128 0x9
	.long	.LASF15
	.byte	0x5
	.value	0x1a4
	.byte	0x1e
	.long	0xd3
	.uleb128 0xb
	.long	.LASF51
	.byte	0x30
	.byte	0x6
	.value	0x153
	.byte	0x8
	.long	0x136
	.uleb128 0xc
	.long	.LASF16
	.byte	0x6
	.value	0x155
	.byte	0x11
	.long	0x17c
	.byte	0
	.uleb128 0xc
	.long	.LASF17
	.byte	0x6
	.value	0x158
	.byte	0x9
	.long	0x87
	.byte	0x8
	.uleb128 0xc
	.long	.LASF18
	.byte	0x6
	.value	0x160
	.byte	0x9
	.long	0x1a1
	.byte	0x10
	.uleb128 0xc
	.long	.LASF19
	.byte	0x6
	.value	0x164
	.byte	0x11
	.long	0x1a7
	.byte	0x18
	.uleb128 0xc
	.long	.LASF20
	.byte	0x6
	.value	0x168
	.byte	0x21
	.long	0x1b7
	.byte	0x20
	.uleb128 0xc
	.long	.LASF21
	.byte	0x6
	.value	0x16f
	.byte	0x11
	.long	0x3c
	.byte	0x28
	.byte	0
	.uleb128 0x9
	.long	.LASF22
	.byte	0x5
	.value	0x1a5
	.byte	0x1a
	.long	0x148
	.uleb128 0x6
	.long	0x136
	.uleb128 0xa
	.long	.LASF24
	.uleb128 0x9
	.long	.LASF25
	.byte	0x5
	.value	0x1a9
	.byte	0x1e
	.long	0x15f
	.uleb128 0x6
	.long	0x14d
	.uleb128 0xa
	.long	.LASF26
	.uleb128 0x9
	.long	.LASF27
	.byte	0x5
	.value	0x1b8
	.byte	0x20
	.long	0x171
	.uleb128 0xa
	.long	.LASF28
	.uleb128 0x7
	.byte	0x8
	.long	0x15a
	.uleb128 0x7
	.byte	0x8
	.long	0x143
	.uleb128 0xd
	.long	0x43
	.long	0x19b
	.uleb128 0xe
	.long	0x19b
	.uleb128 0xe
	.long	0xad
	.uleb128 0xe
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xc6
	.uleb128 0x7
	.byte	0x8
	.long	0x182
	.uleb128 0x7
	.byte	0x8
	.long	0x164
	.uleb128 0xa
	.long	.LASF29
	.uleb128 0x6
	.long	0x1ad
	.uleb128 0x7
	.byte	0x8
	.long	0x1b2
	.uleb128 0x2
	.byte	0x8
	.byte	0x4
	.long	.LASF30
	.uleb128 0x7
	.byte	0x8
	.long	0x90
	.uleb128 0x7
	.byte	0x8
	.long	0xa1
	.uleb128 0x7
	.byte	0x8
	.long	0x95
	.uleb128 0xf
	.long	0x95
	.long	0x1e6
	.uleb128 0x10
	.long	0x3c
	.byte	0x3f
	.byte	0
	.uleb128 0x11
	.long	.LASF35
	.byte	0x7
	.byte	0x6d
	.byte	0x15
	.long	0x1fd
	.uleb128 0xe
	.long	0x87
	.uleb128 0xe
	.long	0x30
	.byte	0
	.uleb128 0x12
	.long	.LASF31
	.byte	0x6
	.byte	0x7f
	.byte	0x14
	.long	0x43
	.long	0x213
	.uleb128 0xe
	.long	0x19b
	.byte	0
	.uleb128 0x12
	.long	.LASF32
	.byte	0x6
	.byte	0xc0
	.byte	0x14
	.long	0x43
	.long	0x233
	.uleb128 0xe
	.long	0x19b
	.uleb128 0xe
	.long	0x1d0
	.uleb128 0xe
	.long	0x233
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x66
	.uleb128 0x12
	.long	.LASF33
	.byte	0x6
	.byte	0xa6
	.byte	0x14
	.long	0x43
	.long	0x259
	.uleb128 0xe
	.long	0x19b
	.uleb128 0xe
	.long	0xad
	.uleb128 0xe
	.long	0x30
	.byte	0
	.uleb128 0x12
	.long	.LASF34
	.byte	0x6
	.byte	0x9d
	.byte	0x14
	.long	0x43
	.long	0x279
	.uleb128 0xe
	.long	0x19b
	.uleb128 0xe
	.long	0x17c
	.uleb128 0xe
	.long	0x279
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xb4
	.uleb128 0x11
	.long	.LASF36
	.byte	0x6
	.byte	0x76
	.byte	0x15
	.long	0x291
	.uleb128 0xe
	.long	0x19b
	.byte	0
	.uleb128 0x13
	.long	.LASF37
	.byte	0x8
	.byte	0x43
	.byte	0xd
	.long	0x2b2
	.uleb128 0xe
	.long	0x1c4
	.uleb128 0xe
	.long	0x1c4
	.uleb128 0xe
	.long	0x66
	.uleb128 0xe
	.long	0x1c4
	.byte	0
	.uleb128 0x14
	.long	.LASF38
	.byte	0x9
	.value	0x147
	.byte	0x19
	.long	0x66
	.long	0x2c9
	.uleb128 0xe
	.long	0x176
	.byte	0
	.uleb128 0x14
	.long	.LASF39
	.byte	0x9
	.value	0x143
	.byte	0x19
	.long	0x66
	.long	0x2e0
	.uleb128 0xe
	.long	0x176
	.byte	0
	.uleb128 0x15
	.long	.LASF52
	.byte	0x1
	.byte	0x42
	.byte	0x5
	.long	0x43
	.quad	.LFB171
	.quad	.LFE171-.LFB171
	.uleb128 0x1
	.byte	0x9c
	.long	0x418
	.uleb128 0x16
	.long	.LASF40
	.byte	0x1
	.byte	0x42
	.byte	0x26
	.long	0x176
	.uleb128 0x3
	.byte	0x91
	.sleb128 -184
	.uleb128 0x17
	.string	"md"
	.byte	0x1
	.byte	0x42
	.byte	0x3a
	.long	0x17c
	.uleb128 0x3
	.byte	0x91
	.sleb128 -192
	.uleb128 0x16
	.long	.LASF41
	.byte	0x1
	.byte	0x43
	.byte	0x23
	.long	0x1ca
	.uleb128 0x3
	.byte	0x91
	.sleb128 -200
	.uleb128 0x16
	.long	.LASF42
	.byte	0x1
	.byte	0x43
	.byte	0x38
	.long	0x1ca
	.uleb128 0x3
	.byte	0x91
	.sleb128 -208
	.uleb128 0x16
	.long	.LASF43
	.byte	0x1
	.byte	0x43
	.byte	0x45
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -216
	.uleb128 0x16
	.long	.LASF44
	.byte	0x1
	.byte	0x44
	.byte	0x1d
	.long	0x66
	.uleb128 0x3
	.byte	0x91
	.sleb128 -220
	.uleb128 0x17
	.string	"key"
	.byte	0x1
	.byte	0x44
	.byte	0x2d
	.long	0x1d0
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x17
	.string	"iv"
	.byte	0x1
	.byte	0x44
	.byte	0x3b
	.long	0x1d0
	.uleb128 0x2
	.byte	0x91
	.sleb128 8
	.uleb128 0x18
	.string	"c"
	.byte	0x1
	.byte	0x45
	.byte	0xe
	.long	0xc6
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x19
	.long	.LASF45
	.byte	0x1
	.byte	0x46
	.byte	0xb
	.long	0x1d6
	.uleb128 0x3
	.byte	0x91
	.sleb128 -160
	.uleb128 0x19
	.long	.LASF46
	.byte	0x1
	.byte	0x47
	.byte	0xc
	.long	0x66
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x18
	.string	"mds"
	.byte	0x1
	.byte	0x48
	.byte	0xc
	.long	0x66
	.uleb128 0x3
	.byte	0x91
	.sleb128 -164
	.uleb128 0x18
	.string	"i"
	.byte	0x1
	.byte	0x48
	.byte	0x15
	.long	0x66
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x18
	.string	"rv"
	.byte	0x1
	.byte	0x49
	.byte	0x7
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.uleb128 0x19
	.long	.LASF47
	.byte	0x1
	.byte	0x4b
	.byte	0xc
	.long	0x66
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x18
	.string	"niv"
	.byte	0x1
	.byte	0x4c
	.byte	0xc
	.long	0x66
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.uleb128 0x1a
	.long	.LASF53
	.long	0x428
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.0
	.uleb128 0x1b
	.string	"err"
	.byte	0x1
	.byte	0x93
	.byte	0x1
	.quad	.L7
	.byte	0
	.uleb128 0xf
	.long	0x90
	.long	0x428
	.uleb128 0x10
	.long	0x3c
	.byte	0x1c
	.byte	0
	.uleb128 0x6
	.long	0x418
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
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3c
	.uleb128 0x19
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
	.uleb128 0xe
	.uleb128 0x5
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0xf
	.uleb128 0x1
	.byte	0x1
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x10
	.uleb128 0x21
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2f
	.uleb128 0xb
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
	.uleb128 0x3c
	.uleb128 0x19
	.uleb128 0x1
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
	.uleb128 0x49
	.uleb128 0x13
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
	.uleb128 0x16
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
	.uleb128 0x19
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
	.uleb128 0x1a
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
	.uleb128 0x1b
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
	.byte	0
	.section	.debug_aranges,"",@progbits
	.long	0x2c
	.value	0x2
	.long	.Ldebug_info0
	.byte	0x8
	.byte	0
	.value	0
	.value	0
	.quad	.LFB171
	.quad	.LFE171-.LFB171
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LFB171
	.quad	.LFE171
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF38:
	.string	"aws_lc_0_38_0_EVP_CIPHER_iv_length"
.LASF15:
	.string	"EVP_MD_CTX"
.LASF41:
	.string	"salt"
.LASF10:
	.string	"short int"
.LASF8:
	.string	"size_t"
.LASF53:
	.string	"__PRETTY_FUNCTION__"
.LASF32:
	.string	"aws_lc_0_38_0_EVP_DigestFinal_ex"
.LASF51:
	.string	"env_md_ctx_st"
.LASF35:
	.string	"aws_lc_0_38_0_OPENSSL_cleanse"
.LASF24:
	.string	"env_md_st"
.LASF12:
	.string	"uint8_t"
.LASF52:
	.string	"aws_lc_0_38_0_EVP_BytesToKey"
.LASF14:
	.string	"ENGINE"
.LASF36:
	.string	"aws_lc_0_38_0_EVP_MD_CTX_init"
.LASF26:
	.string	"evp_cipher_st"
.LASF2:
	.string	"long long int"
.LASF18:
	.string	"update"
.LASF0:
	.string	"long int"
.LASF9:
	.string	"__uint8_t"
.LASF33:
	.string	"aws_lc_0_38_0_EVP_DigestUpdate"
.LASF43:
	.string	"data_len"
.LASF31:
	.string	"aws_lc_0_38_0_EVP_MD_CTX_cleanup"
.LASF27:
	.string	"EVP_PKEY_CTX"
.LASF48:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF3:
	.string	"long double"
.LASF4:
	.string	"unsigned char"
.LASF34:
	.string	"aws_lc_0_38_0_EVP_DigestInit_ex"
.LASF7:
	.string	"signed char"
.LASF21:
	.string	"flags"
.LASF13:
	.string	"long long unsigned int"
.LASF40:
	.string	"type"
.LASF6:
	.string	"unsigned int"
.LASF45:
	.string	"md_buf"
.LASF22:
	.string	"EVP_MD"
.LASF5:
	.string	"short unsigned int"
.LASF16:
	.string	"digest"
.LASF23:
	.string	"engine_st"
.LASF11:
	.string	"char"
.LASF29:
	.string	"evp_md_pctx_ops"
.LASF28:
	.string	"evp_pkey_ctx_st"
.LASF19:
	.string	"pctx"
.LASF42:
	.string	"data"
.LASF1:
	.string	"long unsigned int"
.LASF30:
	.string	"double"
.LASF20:
	.string	"pctx_ops"
.LASF44:
	.string	"count"
.LASF17:
	.string	"md_data"
.LASF47:
	.string	"nkey"
.LASF25:
	.string	"EVP_CIPHER"
.LASF50:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF49:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/derive_key.c"
.LASF46:
	.string	"addmd"
.LASF39:
	.string	"aws_lc_0_38_0_EVP_CIPHER_key_length"
.LASF37:
	.string	"__assert_fail"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
