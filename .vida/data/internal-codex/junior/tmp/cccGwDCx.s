	.file	"cipher_extra.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/cipher_extra.c"
	.section	.rodata
.LC0:
	.string	"aes-128-cbc"
.LC1:
	.string	"aes-128-cfb"
.LC2:
	.string	"aes-128-ctr"
.LC3:
	.string	"aes-128-ecb"
.LC4:
	.string	"aes-128-gcm"
.LC5:
	.string	"aes-128-ofb"
.LC6:
	.string	"aes-192-cbc"
.LC7:
	.string	"aes-192-cfb"
.LC8:
	.string	"aes-192-ctr"
.LC9:
	.string	"aes-192-ecb"
.LC10:
	.string	"aes-192-gcm"
.LC11:
	.string	"aes-192-ofb"
.LC12:
	.string	"aes-256-cbc"
.LC13:
	.string	"aes-256-cfb"
.LC14:
	.string	"aes-256-ctr"
.LC15:
	.string	"aes-256-ecb"
.LC16:
	.string	"aes-256-gcm"
.LC17:
	.string	"aes-256-ofb"
.LC18:
	.string	"aes-256-xts"
.LC19:
	.string	"chacha20-poly1305"
.LC20:
	.string	"des-cbc"
.LC21:
	.string	"des-ecb"
.LC22:
	.string	"des-ede-cbc"
.LC23:
	.string	"des-ede"
.LC24:
	.string	"des-ede3-cbc"
.LC25:
	.string	"rc2-cbc"
.LC26:
	.string	"rc4"
.LC27:
	.string	"bf-cbc"
.LC28:
	.string	"bf-cfb"
.LC29:
	.string	"bf-ecb"
	.section	.data.rel.ro.kCiphers,"aw"
	.align 32
	.type	kCiphers, @object
	.size	kCiphers, 720
kCiphers:
	.long	419
	.zero	4
	.quad	.LC0
	.quad	aws_lc_0_38_0_EVP_aes_128_cbc
	.long	421
	.zero	4
	.quad	.LC1
	.quad	aws_lc_0_38_0_EVP_aes_128_cfb
	.long	904
	.zero	4
	.quad	.LC2
	.quad	aws_lc_0_38_0_EVP_aes_128_ctr
	.long	418
	.zero	4
	.quad	.LC3
	.quad	aws_lc_0_38_0_EVP_aes_128_ecb
	.long	895
	.zero	4
	.quad	.LC4
	.quad	aws_lc_0_38_0_EVP_aes_128_gcm
	.long	420
	.zero	4
	.quad	.LC5
	.quad	aws_lc_0_38_0_EVP_aes_128_ofb
	.long	423
	.zero	4
	.quad	.LC6
	.quad	aws_lc_0_38_0_EVP_aes_192_cbc
	.long	425
	.zero	4
	.quad	.LC7
	.quad	aws_lc_0_38_0_EVP_aes_192_cfb
	.long	905
	.zero	4
	.quad	.LC8
	.quad	aws_lc_0_38_0_EVP_aes_192_ctr
	.long	422
	.zero	4
	.quad	.LC9
	.quad	aws_lc_0_38_0_EVP_aes_192_ecb
	.long	898
	.zero	4
	.quad	.LC10
	.quad	aws_lc_0_38_0_EVP_aes_192_gcm
	.long	424
	.zero	4
	.quad	.LC11
	.quad	aws_lc_0_38_0_EVP_aes_192_ofb
	.long	427
	.zero	4
	.quad	.LC12
	.quad	aws_lc_0_38_0_EVP_aes_256_cbc
	.long	429
	.zero	4
	.quad	.LC13
	.quad	aws_lc_0_38_0_EVP_aes_256_cfb
	.long	906
	.zero	4
	.quad	.LC14
	.quad	aws_lc_0_38_0_EVP_aes_256_ctr
	.long	426
	.zero	4
	.quad	.LC15
	.quad	aws_lc_0_38_0_EVP_aes_256_ecb
	.long	901
	.zero	4
	.quad	.LC16
	.quad	aws_lc_0_38_0_EVP_aes_256_gcm
	.long	428
	.zero	4
	.quad	.LC17
	.quad	aws_lc_0_38_0_EVP_aes_256_ofb
	.long	914
	.zero	4
	.quad	.LC18
	.quad	aws_lc_0_38_0_EVP_aes_256_xts
	.long	950
	.zero	4
	.quad	.LC19
	.quad	aws_lc_0_38_0_EVP_chacha20_poly1305
	.long	31
	.zero	4
	.quad	.LC20
	.quad	aws_lc_0_38_0_EVP_des_cbc
	.long	29
	.zero	4
	.quad	.LC21
	.quad	aws_lc_0_38_0_EVP_des_ecb
	.long	43
	.zero	4
	.quad	.LC22
	.quad	aws_lc_0_38_0_EVP_des_ede_cbc
	.long	32
	.zero	4
	.quad	.LC23
	.quad	aws_lc_0_38_0_EVP_des_ede
	.long	44
	.zero	4
	.quad	.LC24
	.quad	aws_lc_0_38_0_EVP_des_ede3_cbc
	.long	37
	.zero	4
	.quad	.LC25
	.quad	aws_lc_0_38_0_EVP_rc2_cbc
	.long	5
	.zero	4
	.quad	.LC26
	.quad	aws_lc_0_38_0_EVP_rc4
	.long	91
	.zero	4
	.quad	.LC27
	.quad	aws_lc_0_38_0_EVP_bf_cbc
	.long	93
	.zero	4
	.quad	.LC28
	.quad	aws_lc_0_38_0_EVP_bf_cfb
	.long	92
	.zero	4
	.quad	.LC29
	.quad	aws_lc_0_38_0_EVP_bf_ecb
	.section	.rodata
.LC30:
	.string	"3des"
.LC31:
	.string	"DES"
.LC32:
	.string	"aes256"
.LC33:
	.string	"aes128"
.LC34:
	.string	"id-aes128-gcm"
.LC35:
	.string	"id-aes192-gcm"
.LC36:
	.string	"id-aes256-gcm"
	.section	.data.rel.ro.local.kCipherAliases,"aw"
	.align 32
	.type	kCipherAliases, @object
	.size	kCipherAliases, 112
kCipherAliases:
	.quad	.LC30
	.quad	.LC24
	.quad	.LC31
	.quad	.LC20
	.quad	.LC32
	.quad	.LC12
	.quad	.LC33
	.quad	.LC0
	.quad	.LC34
	.quad	.LC4
	.quad	.LC35
	.quad	.LC10
	.quad	.LC36
	.quad	.LC16
	.section	.text.aws_lc_0_38_0_EVP_get_cipherbynid,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_get_cipherbynid
	.type	aws_lc_0_38_0_EVP_get_cipherbynid, @function
aws_lc_0_38_0_EVP_get_cipherbynid:
.LFB151:
	.loc 1 120 48
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movl	%edi, -20(%rbp)
.LBB2:
	.loc 1 121 15
	movq	$0, -8(%rbp)
	.loc 1 121 3
	jmp	.L2
.L5:
	.loc 1 122 20
	movq	-8(%rbp), %rdx
	movq	%rdx, %rax
	addq	%rax, %rax
	addq	%rdx, %rax
	salq	$3, %rax
	movq	%rax, %rdx
	leaq	kCiphers(%rip), %rax
	movl	(%rdx,%rax), %eax
	.loc 1 122 8
	cmpl	%eax, -20(%rbp)
	jne	.L3
	.loc 1 123 25
	movq	-8(%rbp), %rdx
	movq	%rdx, %rax
	addq	%rax, %rax
	addq	%rdx, %rax
	salq	$3, %rax
	movq	%rax, %rdx
	leaq	16+kCiphers(%rip), %rax
	movq	(%rdx,%rax), %rax
	.loc 1 123 14
	call	*%rax
.LVL0:
	jmp	.L4
.L3:
	.loc 1 121 57 discriminator 2
	addq	$1, -8(%rbp)
.L2:
	.loc 1 121 24 discriminator 1
	cmpq	$29, -8(%rbp)
	jbe	.L5
.LBE2:
	.loc 1 126 10
	movl	$0, %eax
.L4:
	.loc 1 127 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE151:
	.size	aws_lc_0_38_0_EVP_get_cipherbynid, .-aws_lc_0_38_0_EVP_get_cipherbynid
	.section	.text.get_cipherbyname,"ax",@progbits
	.type	get_cipherbyname, @function
get_cipherbyname:
.LFB152:
	.loc 1 129 61
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
.LBB3:
	.loc 1 130 15
	movq	$0, -8(%rbp)
	.loc 1 130 3
	jmp	.L7
.L10:
	.loc 1 131 39
	movq	-8(%rbp), %rdx
	movq	%rdx, %rax
	addq	%rax, %rax
	addq	%rdx, %rax
	salq	$3, %rax
	movq	%rax, %rdx
	leaq	8+kCiphers(%rip), %rax
	movq	(%rdx,%rax), %rax
	.loc 1 131 9
	movq	-24(%rbp), %rdx
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_strcasecmp@PLT
	.loc 1 131 8 discriminator 1
	testl	%eax, %eax
	jne	.L8
	.loc 1 132 25
	movq	-8(%rbp), %rdx
	movq	%rdx, %rax
	addq	%rax, %rax
	addq	%rdx, %rax
	salq	$3, %rax
	movq	%rax, %rdx
	leaq	16+kCiphers(%rip), %rax
	movq	(%rdx,%rax), %rax
	.loc 1 132 14
	call	*%rax
.LVL1:
	jmp	.L9
.L8:
	.loc 1 130 57 discriminator 2
	addq	$1, -8(%rbp)
.L7:
	.loc 1 130 24 discriminator 1
	cmpq	$29, -8(%rbp)
	jbe	.L10
.LBE3:
	.loc 1 136 10
	movl	$0, %eax
.L9:
	.loc 1 137 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE152:
	.size	get_cipherbyname, .-get_cipherbyname
	.section	.rodata
	.align 8
.LC37:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/cipher_extra.c"
.LC38:
	.string	"cipher != NULL"
	.section	.text.aws_lc_0_38_0_EVP_get_cipherbyname,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_get_cipherbyname
	.type	aws_lc_0_38_0_EVP_get_cipherbyname, @function
aws_lc_0_38_0_EVP_get_cipherbyname:
.LFB153:
	.loc 1 139 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -40(%rbp)
	.loc 1 140 6
	cmpq	$0, -40(%rbp)
	jne	.L12
	.loc 1 141 12
	movl	$0, %eax
	jmp	.L13
.L12:
	.loc 1 144 27
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	get_cipherbyname
	movq	%rax, -16(%rbp)
	.loc 1 145 6
	cmpq	$0, -16(%rbp)
	je	.L14
	.loc 1 146 12
	movq	-16(%rbp), %rax
	jmp	.L13
.L14:
.LBB4:
	.loc 1 152 14
	movq	$0, -8(%rbp)
	.loc 1 152 3
	jmp	.L15
.L18:
	.loc 1 153 51
	movq	-8(%rbp), %rax
	salq	$4, %rax
	movq	%rax, %rdx
	leaq	kCipherAliases(%rip), %rax
	movq	(%rdx,%rax), %rdx
	.loc 1 153 9
	movq	-40(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_strcasecmp@PLT
	.loc 1 153 8 discriminator 1
	testl	%eax, %eax
	jne	.L16
.LBB5:
	.loc 1 154 12
	movq	-8(%rbp), %rax
	salq	$4, %rax
	movq	%rax, %rdx
	leaq	8+kCipherAliases(%rip), %rax
	movq	(%rdx,%rax), %rax
	movq	%rax, -40(%rbp)
	.loc 1 155 35
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	get_cipherbyname
	movq	%rax, -24(%rbp)
	.loc 1 156 7
	cmpq	$0, -24(%rbp)
	jne	.L17
	.loc 1 156 7 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$156, %edx
	leaq	.LC37(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC38(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L17:
	.loc 1 157 14 is_stmt 1
	movq	-24(%rbp), %rax
	jmp	.L13
.L16:
.LBE5:
	.loc 1 152 62 discriminator 2
	addq	$1, -8(%rbp)
.L15:
	.loc 1 152 23 discriminator 1
	cmpq	$6, -8(%rbp)
	jbe	.L18
.LBE4:
	.loc 1 161 10
	movl	$0, %eax
.L13:
	.loc 1 162 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE153:
	.size	aws_lc_0_38_0_EVP_get_cipherbyname, .-aws_lc_0_38_0_EVP_get_cipherbyname
	.section	.rodata.__PRETTY_FUNCTION__.0,"a"
	.align 32
	.type	__PRETTY_FUNCTION__.0, @object
	.size	__PRETTY_FUNCTION__.0, 35
__PRETTY_FUNCTION__.0:
	.string	"aws_lc_0_38_0_EVP_get_cipherbyname"
	.text
.Letext0:
	.file 2 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 3 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.file 4 "/usr/include/assert.h"
	.file 5 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/mem.h"
	.file 6 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/cipher.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x481
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF55
	.byte	0xc
	.long	.LASF56
	.long	.LASF57
	.long	.Ldebug_ranges0+0
	.quad	0
	.long	.Ldebug_line0
	.uleb128 0x2
	.byte	0x8
	.byte	0x5
	.long	.LASF0
	.uleb128 0x3
	.long	.LASF11
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
	.uleb128 0x2
	.byte	0x2
	.byte	0x5
	.long	.LASF8
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF9
	.uleb128 0x5
	.long	0x7b
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF10
	.uleb128 0x6
	.long	.LASF12
	.byte	0x3
	.value	0x1a9
	.byte	0x1e
	.long	0xa0
	.uleb128 0x5
	.long	0x8e
	.uleb128 0x7
	.long	.LASF58
	.uleb128 0x8
	.byte	0x8
	.long	0x9b
	.uleb128 0x8
	.byte	0x8
	.long	0x82
	.uleb128 0x2
	.byte	0x8
	.byte	0x4
	.long	.LASF13
	.uleb128 0x2
	.byte	0x10
	.byte	0x5
	.long	.LASF14
	.uleb128 0x2
	.byte	0x10
	.byte	0x7
	.long	.LASF15
	.uleb128 0x2
	.byte	0x1
	.byte	0x2
	.long	.LASF16
	.uleb128 0x9
	.byte	0x18
	.byte	0x1
	.byte	0x46
	.byte	0xe
	.long	0xfe
	.uleb128 0xa
	.string	"nid"
	.byte	0x1
	.byte	0x47
	.byte	0x7
	.long	0x43
	.byte	0
	.uleb128 0xb
	.long	.LASF17
	.byte	0x1
	.byte	0x48
	.byte	0xf
	.long	0xab
	.byte	0x8
	.uleb128 0xb
	.long	.LASF18
	.byte	0x1
	.byte	0x49
	.byte	0x17
	.long	0x108
	.byte	0x10
	.byte	0
	.uleb128 0x5
	.long	0xcd
	.uleb128 0xc
	.long	0xa5
	.uleb128 0x8
	.byte	0x8
	.long	0x103
	.uleb128 0xd
	.long	0xfe
	.long	0x11e
	.uleb128 0xe
	.long	0x3c
	.byte	0x1d
	.byte	0
	.uleb128 0x5
	.long	0x10e
	.uleb128 0xf
	.long	.LASF20
	.byte	0x1
	.byte	0x4a
	.byte	0x3
	.long	0x11e
	.uleb128 0x9
	.byte	0x3
	.quad	kCiphers
	.uleb128 0x9
	.byte	0x10
	.byte	0x1
	.byte	0x6b
	.byte	0xe
	.long	0x15d
	.uleb128 0xb
	.long	.LASF19
	.byte	0x1
	.byte	0x6c
	.byte	0xf
	.long	0xab
	.byte	0
	.uleb128 0xb
	.long	.LASF17
	.byte	0x1
	.byte	0x6d
	.byte	0xf
	.long	0xab
	.byte	0x8
	.byte	0
	.uleb128 0x5
	.long	0x139
	.uleb128 0xd
	.long	0x15d
	.long	0x172
	.uleb128 0xe
	.long	0x3c
	.byte	0x6
	.byte	0
	.uleb128 0x5
	.long	0x162
	.uleb128 0xf
	.long	.LASF21
	.byte	0x1
	.byte	0x6e
	.byte	0x3
	.long	0x172
	.uleb128 0x9
	.byte	0x3
	.quad	kCipherAliases
	.uleb128 0x10
	.long	.LASF22
	.byte	0x4
	.byte	0x43
	.byte	0xd
	.long	0x1ae
	.uleb128 0x11
	.long	0xab
	.uleb128 0x11
	.long	0xab
	.uleb128 0x11
	.long	0x66
	.uleb128 0x11
	.long	0xab
	.byte	0
	.uleb128 0x12
	.long	.LASF23
	.byte	0x5
	.byte	0xa9
	.byte	0x14
	.long	0x43
	.long	0x1c9
	.uleb128 0x11
	.long	0xab
	.uleb128 0x11
	.long	0xab
	.byte	0
	.uleb128 0x13
	.long	.LASF24
	.byte	0x6
	.value	0x223
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF25
	.byte	0x6
	.value	0x229
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF26
	.byte	0x6
	.value	0x226
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF27
	.byte	0x6
	.byte	0x4b
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF28
	.byte	0x6
	.byte	0x6a
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF29
	.byte	0x6
	.byte	0x52
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF30
	.byte	0x6
	.byte	0x4f
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF31
	.byte	0x6
	.byte	0x51
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF32
	.byte	0x6
	.byte	0x4e
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF33
	.byte	0x6
	.byte	0x4d
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF34
	.byte	0x6
	.byte	0x77
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF35
	.byte	0x6
	.byte	0x5d
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF36
	.byte	0x6
	.byte	0x5c
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF37
	.byte	0x6
	.value	0x1ee
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF38
	.byte	0x6
	.byte	0x59
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF39
	.byte	0x6
	.byte	0x5b
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF40
	.byte	0x6
	.value	0x21a
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF41
	.byte	0x6
	.byte	0x5a
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF42
	.byte	0x6
	.value	0x1f9
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF43
	.byte	0x6
	.value	0x1f8
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF44
	.byte	0x6
	.value	0x1f5
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF45
	.byte	0x6
	.value	0x1f7
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF46
	.byte	0x6
	.value	0x20e
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF47
	.byte	0x6
	.value	0x1f6
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF48
	.byte	0x6
	.byte	0x57
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF49
	.byte	0x6
	.value	0x1ed
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF50
	.byte	0x6
	.byte	0x54
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF51
	.byte	0x6
	.byte	0x56
	.byte	0x22
	.long	0xa5
	.uleb128 0x13
	.long	.LASF52
	.byte	0x6
	.value	0x202
	.byte	0x22
	.long	0xa5
	.uleb128 0x14
	.long	.LASF53
	.byte	0x6
	.byte	0x55
	.byte	0x22
	.long	0xa5
	.uleb128 0x15
	.long	.LASF59
	.byte	0x1
	.byte	0x8b
	.byte	0x13
	.long	0xa5
	.quad	.LFB153
	.quad	.LFE153-.LFB153
	.uleb128 0x1
	.byte	0x9c
	.long	0x3d1
	.uleb128 0x16
	.long	.LASF17
	.byte	0x1
	.byte	0x8b
	.byte	0x34
	.long	0xab
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x17
	.string	"ec"
	.byte	0x1
	.byte	0x90
	.byte	0x16
	.long	0xa5
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x18
	.long	.LASF60
	.long	0x3e1
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.0
	.uleb128 0x19
	.quad	.LBB4
	.quad	.LBE4-.LBB4
	.uleb128 0x17
	.string	"i"
	.byte	0x1
	.byte	0x98
	.byte	0xe
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x19
	.quad	.LBB5
	.quad	.LBE5-.LBB5
	.uleb128 0xf
	.long	.LASF54
	.byte	0x1
	.byte	0x9b
	.byte	0x1a
	.long	0xa5
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.byte	0
	.byte	0
	.uleb128 0xd
	.long	0x82
	.long	0x3e1
	.uleb128 0xe
	.long	0x3c
	.byte	0x22
	.byte	0
	.uleb128 0x5
	.long	0x3d1
	.uleb128 0x1a
	.long	.LASF61
	.byte	0x1
	.byte	0x81
	.byte	0x1a
	.long	0xa5
	.quad	.LFB152
	.quad	.LFE152-.LFB152
	.uleb128 0x1
	.byte	0x9c
	.long	0x437
	.uleb128 0x16
	.long	.LASF17
	.byte	0x1
	.byte	0x81
	.byte	0x37
	.long	0xab
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x19
	.quad	.LBB3
	.quad	.LBE3-.LBB3
	.uleb128 0x17
	.string	"i"
	.byte	0x1
	.byte	0x82
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.byte	0
	.uleb128 0x1b
	.long	.LASF62
	.byte	0x1
	.byte	0x78
	.byte	0x13
	.long	0xa5
	.quad	.LFB151
	.quad	.LFE151-.LFB151
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x1c
	.string	"nid"
	.byte	0x1
	.byte	0x78
	.byte	0x2b
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.uleb128 0x19
	.quad	.LBB2
	.quad	.LBE2-.LBB2
	.uleb128 0x17
	.string	"i"
	.byte	0x1
	.byte	0x79
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
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
	.uleb128 0x13
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3c
	.uleb128 0x19
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
	.uleb128 0x13
	.byte	0x1
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
	.uleb128 0xa
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
	.uleb128 0x15
	.byte	0
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0xd
	.uleb128 0x1
	.byte	0x1
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0xe
	.uleb128 0x21
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2f
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0xf
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
	.uleb128 0x87
	.uleb128 0x19
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
	.byte	0
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
	.byte	0
	.byte	0
	.uleb128 0x14
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
	.uleb128 0x3c
	.uleb128 0x19
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
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x34
	.uleb128 0x19
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
	.uleb128 0x2116
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
	.uleb128 0x1c
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
	.byte	0
	.section	.debug_aranges,"",@progbits
	.long	0x4c
	.value	0x2
	.long	.Ldebug_info0
	.byte	0x8
	.byte	0
	.value	0
	.value	0
	.quad	.LFB151
	.quad	.LFE151-.LFB151
	.quad	.LFB152
	.quad	.LFE152-.LFB152
	.quad	.LFB153
	.quad	.LFE153-.LFB153
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LFB151
	.quad	.LFE151
	.quad	.LFB152
	.quad	.LFE152
	.quad	.LFB153
	.quad	.LFE153
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF24:
	.string	"aws_lc_0_38_0_EVP_bf_ecb"
.LASF25:
	.string	"aws_lc_0_38_0_EVP_bf_cfb"
.LASF54:
	.string	"cipher"
.LASF41:
	.string	"aws_lc_0_38_0_EVP_aes_256_cbc"
.LASF8:
	.string	"short int"
.LASF11:
	.string	"size_t"
.LASF49:
	.string	"aws_lc_0_38_0_EVP_aes_128_gcm"
.LASF38:
	.string	"aws_lc_0_38_0_EVP_aes_256_ecb"
.LASF47:
	.string	"aws_lc_0_38_0_EVP_aes_192_cbc"
.LASF60:
	.string	"__PRETTY_FUNCTION__"
.LASF51:
	.string	"aws_lc_0_38_0_EVP_aes_128_ctr"
.LASF33:
	.string	"aws_lc_0_38_0_EVP_des_cbc"
.LASF2:
	.string	"long long int"
.LASF44:
	.string	"aws_lc_0_38_0_EVP_aes_192_ecb"
.LASF32:
	.string	"aws_lc_0_38_0_EVP_des_ecb"
.LASF46:
	.string	"aws_lc_0_38_0_EVP_aes_192_cfb"
.LASF56:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/cipher_extra.c"
.LASF19:
	.string	"alias"
.LASF34:
	.string	"aws_lc_0_38_0_EVP_chacha20_poly1305"
.LASF23:
	.string	"aws_lc_0_38_0_OPENSSL_strcasecmp"
.LASF53:
	.string	"aws_lc_0_38_0_EVP_aes_128_cbc"
.LASF58:
	.string	"evp_cipher_st"
.LASF50:
	.string	"aws_lc_0_38_0_EVP_aes_128_ecb"
.LASF18:
	.string	"func"
.LASF15:
	.string	"__int128 unsigned"
.LASF52:
	.string	"aws_lc_0_38_0_EVP_aes_128_cfb"
.LASF0:
	.string	"long int"
.LASF17:
	.string	"name"
.LASF30:
	.string	"aws_lc_0_38_0_EVP_des_ede"
.LASF55:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF3:
	.string	"long double"
.LASF4:
	.string	"unsigned char"
.LASF20:
	.string	"kCiphers"
.LASF7:
	.string	"signed char"
.LASF10:
	.string	"long long unsigned int"
.LASF35:
	.string	"aws_lc_0_38_0_EVP_aes_256_xts"
.LASF6:
	.string	"unsigned int"
.LASF40:
	.string	"aws_lc_0_38_0_EVP_aes_256_cfb"
.LASF5:
	.string	"short unsigned int"
.LASF31:
	.string	"aws_lc_0_38_0_EVP_des_ede_cbc"
.LASF9:
	.string	"char"
.LASF29:
	.string	"aws_lc_0_38_0_EVP_des_ede3_cbc"
.LASF61:
	.string	"get_cipherbyname"
.LASF37:
	.string	"aws_lc_0_38_0_EVP_aes_256_gcm"
.LASF16:
	.string	"_Bool"
.LASF59:
	.string	"aws_lc_0_38_0_EVP_get_cipherbyname"
.LASF14:
	.string	"__int128"
.LASF1:
	.string	"long unsigned int"
.LASF13:
	.string	"double"
.LASF36:
	.string	"aws_lc_0_38_0_EVP_aes_256_ofb"
.LASF42:
	.string	"aws_lc_0_38_0_EVP_aes_192_ofb"
.LASF27:
	.string	"aws_lc_0_38_0_EVP_rc4"
.LASF21:
	.string	"kCipherAliases"
.LASF12:
	.string	"EVP_CIPHER"
.LASF57:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF62:
	.string	"aws_lc_0_38_0_EVP_get_cipherbynid"
.LASF22:
	.string	"__assert_fail"
.LASF48:
	.string	"aws_lc_0_38_0_EVP_aes_128_ofb"
.LASF39:
	.string	"aws_lc_0_38_0_EVP_aes_256_ctr"
.LASF28:
	.string	"aws_lc_0_38_0_EVP_rc2_cbc"
.LASF26:
	.string	"aws_lc_0_38_0_EVP_bf_cbc"
.LASF43:
	.string	"aws_lc_0_38_0_EVP_aes_192_gcm"
.LASF45:
	.string	"aws_lc_0_38_0_EVP_aes_192_ctr"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
