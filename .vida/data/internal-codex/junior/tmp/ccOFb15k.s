	.file	"e_aes_cbc_hmac_sha256.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aes_cbc_hmac_sha256.c"
	.section	.text.OPENSSL_ia32cap_get,"ax",@progbits
	.type	OPENSSL_ia32cap_get, @function
OPENSSL_ia32cap_get:
.LFB194:
	.file 2 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/../fipsmodule/aes/../cpucap/internal.h"
	.loc 2 48 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 2 49 10
	movq	aws_lc_0_38_0_OPENSSL_ia32cap_P@GOTPCREL(%rip), %rax
	.loc 2 50 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE194:
	.size	OPENSSL_ia32cap_get, .-OPENSSL_ia32cap_get
	.section	.text.CRYPTO_is_intel_cpu,"ax",@progbits
	.type	CRYPTO_is_intel_cpu, @function
CRYPTO_is_intel_cpu:
.LFB196:
	.loc 2 59 46
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 2 61 32
	call	OPENSSL_ia32cap_get
	.loc 2 61 32 is_stmt 0 discriminator 1
	movl	(%rax), %eax
	.loc 2 61 36 is_stmt 1 discriminator 1
	andl	$1073741824, %eax
	.loc 2 61 49 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 2 62 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE196:
	.size	CRYPTO_is_intel_cpu, .-CRYPTO_is_intel_cpu
	.section	.text.CRYPTO_is_AESNI_capable,"ax",@progbits
	.type	CRYPTO_is_AESNI_capable, @function
CRYPTO_is_AESNI_capable:
.LFB201:
	.loc 2 82 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 2 83 11
	call	OPENSSL_ia32cap_get
	.loc 2 83 32 discriminator 1
	addq	$4, %rax
	movl	(%rax), %eax
	.loc 2 83 36 discriminator 1
	andl	$33554432, %eax
	.loc 2 83 49 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 2 84 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE201:
	.size	CRYPTO_is_AESNI_capable, .-CRYPTO_is_AESNI_capable
	.section	.text.CRYPTO_is_AMD_XOP_support,"ax",@progbits
	.type	CRYPTO_is_AMD_XOP_support, @function
CRYPTO_is_AMD_XOP_support:
.LFB204:
	.loc 2 97 52
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 2 98 11
	call	OPENSSL_ia32cap_get
	.loc 2 98 32 discriminator 1
	addq	$4, %rax
	movl	(%rax), %eax
	.loc 2 98 36 discriminator 1
	andl	$2048, %eax
	.loc 2 98 49 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 2 99 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE204:
	.size	CRYPTO_is_AMD_XOP_support, .-CRYPTO_is_AMD_XOP_support
	.section	.text.CRYPTO_is_AVX2_capable,"ax",@progbits
	.type	CRYPTO_is_AVX2_capable, @function
CRYPTO_is_AVX2_capable:
.LFB206:
	.loc 2 107 49
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 2 108 11
	call	OPENSSL_ia32cap_get
	.loc 2 108 32 discriminator 1
	addq	$8, %rax
	movl	(%rax), %eax
	.loc 2 108 36 discriminator 1
	andl	$32, %eax
	.loc 2 108 48 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 2 109 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE206:
	.size	CRYPTO_is_AVX2_capable, .-CRYPTO_is_AVX2_capable
	.section	.text.CRYPTO_is_SHAEXT_capable,"ax",@progbits
	.type	CRYPTO_is_SHAEXT_capable, @function
CRYPTO_is_SHAEXT_capable:
.LFB209:
	.loc 2 119 51
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 2 120 11
	call	OPENSSL_ia32cap_get
	.loc 2 120 32 discriminator 1
	addq	$8, %rax
	movl	(%rax), %eax
	.loc 2 120 36 discriminator 1
	andl	$536870912, %eax
	.loc 2 120 49 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 2 121 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE209:
	.size	CRYPTO_is_SHAEXT_capable, .-CRYPTO_is_SHAEXT_capable
	.section	.text.constant_time_msb_w,"ax",@progbits
	.type	constant_time_msb_w, @function
constant_time_msb_w:
.LFB221:
	.file 3 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/../fipsmodule/aes/../../internal.h"
	.loc 3 343 66
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	.loc 3 344 13
	movq	-8(%rbp), %rax
	sarq	$63, %rax
	.loc 3 345 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE221:
	.size	constant_time_msb_w, .-constant_time_msb_w
	.section	.text.constant_time_is_zero_w,"ax",@progbits
	.type	constant_time_is_zero_w, @function
constant_time_is_zero_w:
.LFB226:
	.loc 3 402 70
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$8, %rsp
	movq	%rdi, -8(%rbp)
	.loc 3 414 30
	movq	-8(%rbp), %rax
	notq	%rax
	movq	%rax, %rdx
	.loc 3 414 38
	movq	-8(%rbp), %rax
	subq	$1, %rax
	.loc 3 414 10
	andq	%rdx, %rax
	movq	%rax, %rdi
	call	constant_time_msb_w
	.loc 3 415 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE226:
	.size	constant_time_is_zero_w, .-constant_time_is_zero_w
	.section	.text.constant_time_eq_w,"ax",@progbits
	.type	constant_time_eq_w, @function
constant_time_eq_w:
.LFB228:
	.loc 3 425 65
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 3 426 10
	movq	-8(%rbp), %rax
	xorq	-16(%rbp), %rax
	movq	%rax, %rdi
	call	constant_time_is_zero_w
	.loc 3 427 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE228:
	.size	constant_time_eq_w, .-constant_time_eq_w
	.section	.text.constant_time_eq_int,"ax",@progbits
	.type	constant_time_eq_int, @function
constant_time_eq_int:
.LFB230:
	.loc 3 437 64
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$8, %rsp
	movl	%edi, -4(%rbp)
	movl	%esi, -8(%rbp)
	.loc 3 438 10
	movl	-8(%rbp), %eax
	movslq	%eax, %rdx
	movl	-4(%rbp), %eax
	cltq
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	constant_time_eq_w
	.loc 3 439 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE230:
	.size	constant_time_eq_int, .-constant_time_eq_int
	.section	.text.OPENSSL_memcpy,"ax",@progbits
	.type	OPENSSL_memcpy, @function
OPENSSL_memcpy:
.LFB247:
	.loc 3 956 74
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
	.loc 3 957 6
	cmpq	$0, -24(%rbp)
	jne	.L22
	.loc 3 958 12
	movq	-8(%rbp), %rax
	jmp	.L23
.L22:
	.loc 3 961 10
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	memcpy@PLT
.L23:
	.loc 3 962 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE247:
	.size	OPENSSL_memcpy, .-OPENSSL_memcpy
	.section	.text.OPENSSL_memset,"ax",@progbits
	.type	OPENSSL_memset, @function
OPENSSL_memset:
.LFB249:
	.loc 3 972 64
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
	.loc 3 973 6
	cmpq	$0, -24(%rbp)
	jne	.L25
	.loc 3 974 12
	movq	-8(%rbp), %rax
	jmp	.L26
.L25:
	.loc 3 977 10
	movq	-24(%rbp), %rdx
	movl	-12(%rbp), %ecx
	movq	-8(%rbp), %rax
	movl	%ecx, %esi
	movq	%rax, %rdi
	call	memset@PLT
.L26:
	.loc 3 978 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE249:
	.size	OPENSSL_memset, .-OPENSSL_memset
	.section	.text.hwaes_capable,"ax",@progbits
	.type	hwaes_capable, @function
hwaes_capable:
.LFB282:
	.file 4 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/../fipsmodule/aes/internal.h"
	.loc 4 45 40
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 4 45 49
	call	CRYPTO_is_AESNI_capable
	.loc 4 45 76
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE282:
	.size	hwaes_capable, .-hwaes_capable
	.section	.text.aesni_cbc_hmac_sha256_init_key,"ax",@progbits
	.type	aesni_cbc_hmac_sha256_init_key, @function
aesni_cbc_hmac_sha256_init_key:
.LFB292:
	.loc 1 64 71
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%rbx
	subq	$72, %rsp
	.cfi_offset 3, -24
	movq	%rdi, 24(%rsp)
	movq	%rsi, 16(%rsp)
	movq	%rdx, 8(%rsp)
	movl	%ecx, 4(%rsp)
	.loc 1 65 24
	movq	24(%rsp), %rax
	movq	16(%rax), %rax
	movq	%rax, 48(%rsp)
	.loc 1 68 18
	movq	24(%rsp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_CIPHER_CTX_key_length@PLT
	.loc 1 68 49 discriminator 1
	sall	$3, %eax
	.loc 1 68 7 discriminator 1
	movl	%eax, 44(%rsp)
	.loc 1 69 6
	cmpl	$0, 4(%rsp)
	je	.L30
	.loc 1 70 11
	movq	48(%rsp), %rdx
	movl	44(%rsp), %ecx
	movq	16(%rsp), %rax
	movl	%ecx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes_hw_set_encrypt_key@PLT
	movl	%eax, 60(%rsp)
	jmp	.L31
.L30:
	.loc 1 72 11
	movq	48(%rsp), %rdx
	movl	44(%rsp), %ecx
	movq	16(%rsp), %rax
	movl	%ecx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes_hw_set_decrypt_key@PLT
	movl	%eax, 60(%rsp)
.L31:
	.loc 1 75 3
	movq	48(%rsp), %rax
	addq	$244, %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Init@PLT
	.loc 1 76 13
	movq	48(%rsp), %rax
	movq	48(%rsp), %rdx
	vmovdqu	244(%rdx), %ymm0
	vmovdqu	%ymm0, 356(%rax)
	vmovdqu	276(%rdx), %ymm0
	vmovdqu	%ymm0, 388(%rax)
	vmovdqu	308(%rdx), %ymm0
	vmovdqu	%ymm0, 420(%rax)
	movq	340(%rdx), %rcx
	movq	348(%rdx), %rbx
	movq	%rcx, 452(%rax)
	movq	%rbx, 460(%rax)
	.loc 1 77 11
	movq	48(%rsp), %rax
	movq	48(%rsp), %rdx
	vmovdqu	244(%rdx), %ymm0
	vmovdqu	%ymm0, 468(%rax)
	vmovdqu	276(%rdx), %ymm0
	vmovdqu	%ymm0, 500(%rax)
	vmovdqu	308(%rdx), %ymm0
	vmovdqu	%ymm0, 532(%rax)
	movq	340(%rdx), %rcx
	movq	348(%rdx), %rbx
	movq	%rcx, 564(%rax)
	movq	%rbx, 572(%rax)
	.loc 1 79 23
	movq	48(%rsp), %rax
	movq	$-1, 584(%rax)
	.loc 1 81 22
	movl	60(%rsp), %eax
	notl	%eax
	shrl	$31, %eax
	movzbl	%al, %eax
	.loc 1 82 1
	movq	-8(%rbp), %rbx
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE292:
	.size	aesni_cbc_hmac_sha256_init_key, .-aesni_cbc_hmac_sha256_init_key
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aes_cbc_hmac_sha256.c"
	.align 8
.LC1:
	.string	"mac_len == SHA256_DIGEST_LENGTH"
	.section	.text.aesni_cbc_hmac_sha256_cipher,"ax",@progbits
	.type	aesni_cbc_hmac_sha256_cipher, @function
aesni_cbc_hmac_sha256_cipher:
.LFB293:
	.loc 1 104 72
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%r12
	pushq	%r10
	pushq	%rbx
	subq	$280, %rsp
	.cfi_offset 12, -24
	.cfi_offset 10, -32
	.cfi_offset 3, -40
	leaq	16(%rbp), %r10
	movq	%rdi, -280(%rbp)
	movq	%rsi, -288(%rbp)
	movq	%rdx, -296(%rbp)
	movq	%rcx, -304(%rbp)
	.loc 1 105 24
	movq	-280(%rbp), %rax
	movq	16(%rax), %rax
	movq	%rax, -72(%rbp)
	.loc 1 107 10
	movq	-72(%rbp), %rax
	movq	584(%rax), %rax
	movq	%rax, -40(%rbp)
	.loc 1 107 38
	movq	$0, -48(%rbp)
	.loc 1 108 10
	movq	$0, -56(%rbp)
	.loc 1 110 43
	movq	-72(%rbp), %rax
	movl	572(%rax), %eax
	.loc 1 110 34
	movl	$64, %edx
	subl	%eax, %edx
	.loc 1 110 10
	movl	%edx, %eax
	movq	%rax, -64(%rbp)
	.loc 1 112 23
	movq	-72(%rbp), %rax
	movq	$-1, 584(%rax)
	.loc 1 114 11
	movq	-304(%rbp), %rax
	andl	$15, %eax
	.loc 1 114 6
	testq	%rax, %rax
	je	.L34
	.loc 1 115 5
	movl	$115, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$106, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 116 12
	movl	$0, %eax
	jmp	.L35
.L34:
	.loc 1 119 7
	movq	-280(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_CIPHER_CTX_encrypting@PLT
	.loc 1 119 6 discriminator 1
	testl	%eax, %eax
	je	.L36
	.loc 1 129 8
	cmpq	$-1, -40(%rbp)
	jne	.L37
	.loc 1 132 7
	movl	$132, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$112, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 133 14
	movl	$0, %eax
	jmp	.L35
.L37:
	.loc 1 136 39
	movq	-40(%rbp), %rax
	addq	$48, %rax
	.loc 1 136 57
	andq	$-16, %rax
	.loc 1 135 8
	cmpq	%rax, -304(%rbp)
	je	.L38
	.loc 1 138 7
	movl	$138, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$119, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 139 14
	movl	$0, %eax
	jmp	.L35
.L38:
	.loc 1 140 24
	movq	-72(%rbp), %rax
	movzwl	592(%rax), %eax
	.loc 1 140 15
	cmpw	$769, %ax
	jbe	.L39
	.loc 1 141 14
	movq	$16, -48(%rbp)
.L39:
	.loc 1 155 10
	call	CRYPTO_is_SHAEXT_capable
	.loc 1 155 8 discriminator 1
	testl	%eax, %eax
	jne	.L40
	.loc 1 156 11
	call	CRYPTO_is_AVX2_capable
	.loc 1 155 37 discriminator 1
	testl	%eax, %eax
	je	.L41
	.loc 1 157 12
	call	CRYPTO_is_AMD_XOP_support
	movl	%eax, %ebx
	.loc 1 157 42 discriminator 1
	call	CRYPTO_is_intel_cpu
	.loc 1 157 40 discriminator 2
	orl	%ebx, %eax
	.loc 1 156 36
	testl	%eax, %eax
	je	.L41
.L40:
	.loc 1 158 25
	movq	-64(%rbp), %rdx
	movq	-48(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 157 67
	cmpq	-40(%rbp), %rax
	jnb	.L41
	.loc 1 159 36
	movq	-64(%rbp), %rdx
	movq	-48(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 159 25
	movq	-40(%rbp), %rdx
	subq	%rax, %rdx
	.loc 1 159 17
	movq	%rdx, %rax
	shrq	$6, %rax
	movq	%rax, -104(%rbp)
	.loc 1 158 35
	cmpq	$0, -104(%rbp)
	je	.L41
	.loc 1 162 34
	movq	-296(%rbp), %rdx
	movq	-48(%rbp), %rax
	leaq	(%rdx,%rax), %rsi
	.loc 1 162 7
	movq	-72(%rbp), %rax
	leaq	468(%rax), %rcx
	movq	-64(%rbp), %rax
	movq	%rax, %rdx
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 166 46
	movq	-48(%rbp), %rdx
	movq	-64(%rbp), %rax
	addq	%rax, %rdx
	movq	-296(%rbp), %rax
	leaq	(%rdx,%rax), %rdi
	.loc 1 164 7
	movq	-72(%rbp), %rax
	leaq	468(%rax), %r9
	.loc 1 165 37
	movq	-280(%rbp), %rax
	leaq	52(%rax), %r8
	.loc 1 164 45
	movq	-72(%rbp), %rcx
	.loc 1 164 7
	movq	-104(%rbp), %rdx
	movq	-288(%rbp), %rsi
	movq	-296(%rbp), %rax
	subq	$8, %rsp
	pushq	%rdi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesni_cbc_sha256_enc@PLT
	addq	$16, %rsp
	.loc 1 167 14
	salq	$6, -104(%rbp)
	.loc 1 168 15
	movq	-104(%rbp), %rax
	addq	%rax, -56(%rbp)
	.loc 1 169 15
	movq	-104(%rbp), %rax
	addq	%rax, -64(%rbp)
	.loc 1 170 14
	movq	-72(%rbp), %rax
	movl	504(%rax), %eax
	.loc 1 170 28
	movq	-104(%rbp), %rdx
	shrq	$29, %rdx
	.loc 1 170 18
	addl	%eax, %edx
	movq	-72(%rbp), %rax
	movl	%edx, 504(%rax)
	.loc 1 171 28
	salq	$3, -104(%rbp)
	movq	-104(%rbp), %rdx
	.loc 1 171 14
	movq	-72(%rbp), %rax
	movl	500(%rax), %eax
	.loc 1 171 18
	addl	%eax, %edx
	movq	-72(%rbp), %rax
	movl	%edx, 500(%rax)
	.loc 1 172 18
	movq	-72(%rbp), %rax
	movl	500(%rax), %eax
	.loc 1 172 24
	movq	-104(%rbp), %rdx
	.loc 1 172 10
	cmpl	%edx, %eax
	jnb	.L43
	.loc 1 173 16
	movq	-72(%rbp), %rax
	movl	504(%rax), %eax
	.loc 1 173 19
	leal	1(%rax), %edx
	movq	-72(%rbp), %rax
	movl	%edx, 504(%rax)
	.loc 1 172 10
	jmp	.L43
.L41:
	.loc 1 176 15
	movq	$0, -64(%rbp)
.L43:
	.loc 1 178 13
	movq	-48(%rbp), %rax
	addq	%rax, -64(%rbp)
	.loc 1 179 5
	movq	-40(%rbp), %rax
	subq	-64(%rbp), %rax
	.loc 1 179 32
	movq	-296(%rbp), %rcx
	movq	-64(%rbp), %rdx
	leaq	(%rcx,%rdx), %rsi
	.loc 1 179 5
	movq	-72(%rbp), %rdx
	leaq	468(%rdx), %rcx
	movq	%rax, %rdx
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 181 8
	movq	-296(%rbp), %rax
	cmpq	-288(%rbp), %rax
	je	.L44
	.loc 1 182 7
	movq	-40(%rbp), %rax
	subq	-56(%rbp), %rax
	.loc 1 182 40
	movq	-296(%rbp), %rcx
	movq	-56(%rbp), %rdx
	leaq	(%rcx,%rdx), %rsi
	.loc 1 182 26
	movq	-288(%rbp), %rcx
	movq	-56(%rbp), %rdx
	addq	%rdx, %rcx
	.loc 1 182 7
	movq	%rax, %rdx
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
.L44:
	.loc 1 186 5
	movq	-72(%rbp), %rax
	leaq	468(%rax), %rdx
	movq	-288(%rbp), %rcx
	movq	-40(%rbp), %rax
	addq	%rcx, %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Final@PLT
	.loc 1 187 13
	movq	-72(%rbp), %rax
	movq	-72(%rbp), %rdx
	vmovdqu	356(%rdx), %ymm0
	vmovdqu	%ymm0, 468(%rax)
	vmovdqu	388(%rdx), %ymm0
	vmovdqu	%ymm0, 500(%rax)
	vmovdqu	420(%rdx), %ymm0
	vmovdqu	%ymm0, 532(%rax)
	movq	452(%rdx), %rcx
	movq	460(%rdx), %rbx
	movq	%rcx, 564(%rax)
	movq	%rbx, 572(%rax)
	.loc 1 188 33
	movq	-288(%rbp), %rdx
	movq	-40(%rbp), %rax
	leaq	(%rdx,%rax), %rcx
	.loc 1 188 5
	movq	-72(%rbp), %rax
	addq	$468, %rax
	movl	$32, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 189 5
	movq	-72(%rbp), %rax
	leaq	468(%rax), %rdx
	movq	-288(%rbp), %rcx
	movq	-40(%rbp), %rax
	addq	%rcx, %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Final@PLT
	.loc 1 192 10
	addq	$32, -40(%rbp)
	.loc 1 193 18
	movq	-304(%rbp), %rax
	movl	%eax, %ecx
	movq	-40(%rbp), %rax
	movl	%eax, %edx
	movl	%ecx, %eax
	subl	%edx, %eax
	.loc 1 193 12
	subl	$1, %eax
	movl	%eax, -108(%rbp)
	.loc 1 193 5
	jmp	.L45
.L46:
	.loc 1 194 10
	movq	-288(%rbp), %rdx
	movq	-40(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 194 17
	movl	-108(%rbp), %edx
	movb	%dl, (%rax)
	.loc 1 193 46 discriminator 3
	addq	$1, -40(%rbp)
.L45:
	.loc 1 193 35 discriminator 1
	movq	-40(%rbp), %rax
	cmpq	-304(%rbp), %rax
	jb	.L46
	.loc 1 198 27
	movq	-280(%rbp), %rax
	leaq	52(%rax), %r8
	.loc 1 197 69
	movq	-72(%rbp), %rdx
	.loc 1 197 5
	movq	-304(%rbp), %rax
	subq	-56(%rbp), %rax
	movq	-288(%rbp), %rsi
	movq	-56(%rbp), %rcx
	addq	%rcx, %rsi
	.loc 1 197 28
	movq	-288(%rbp), %rdi
	movq	-56(%rbp), %rcx
	addq	%rcx, %rdi
	.loc 1 197 5
	movl	$1, %r9d
	movq	%rdx, %rcx
	movq	%rax, %rdx
	call	aws_lc_0_38_0_aes_hw_cbc_encrypt@PLT
	.loc 1 199 12
	movl	$1, %eax
	jmp	.L35
.L36:
.LBB2:
	.loc 1 201 8
	cmpq	$13, -40(%rbp)
	je	.L47
	.loc 1 204 7
	movl	$204, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$112, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 205 14
	movl	$0, %eax
	jmp	.L35
.L47:
	.loc 1 212 51
	movq	-280(%rbp), %rax
	leaq	52(%rax), %rdi
	.loc 1 212 38
	movq	-72(%rbp), %rcx
	.loc 1 212 5
	movq	-304(%rbp), %rdx
	movq	-288(%rbp), %rsi
	movq	-296(%rbp), %rax
	movl	$0, %r9d
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes_hw_cbc_encrypt@PLT
	.loc 1 214 32
	movq	-40(%rbp), %rax
	leaq	-4(%rax), %rdx
	.loc 1 214 26
	movq	-72(%rbp), %rax
	movzbl	592(%rax,%rdx), %eax
	movzbl	%al, %eax
	.loc 1 214 37
	sall	$8, %eax
	movl	%eax, %ecx
	.loc 1 214 66
	movq	-40(%rbp), %rax
	leaq	-3(%rax), %rdx
	.loc 1 214 60
	movq	-72(%rbp), %rax
	movzbl	592(%rax,%rdx), %eax
	movzbl	%al, %eax
	.loc 1 214 42
	orl	%ecx, %eax
	.loc 1 214 8
	cmpl	$769, %eax
	jle	.L49
	.loc 1 216 14
	movq	$16, -48(%rbp)
.L49:
	.loc 1 218 46
	movq	-48(%rbp), %rax
	addq	$33, %rax
	.loc 1 218 8
	cmpq	%rax, -304(%rbp)
	jnb	.L50
	.loc 1 219 7
	movl	$219, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$119, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 220 14
	movl	$0, %eax
	jmp	.L35
.L50:
	.loc 1 224 9
	movq	-48(%rbp), %rax
	addq	%rax, -288(%rbp)
	.loc 1 225 9
	movq	-48(%rbp), %rax
	subq	%rax, -304(%rbp)
	.loc 1 232 10
	movq	-304(%rbp), %rcx
	movq	-288(%rbp), %rdx
	leaq	-120(%rbp), %rsi
	leaq	-128(%rbp), %rax
	movl	$32, %r9d
	movl	$16, %r8d
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_tls_cbc_remove_padding@PLT
	.loc 1 232 8 discriminator 1
	testl	%eax, %eax
	jne	.L51
	.loc 1 235 7
	movl	$235, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 236 14
	movl	$0, %eax
	jmp	.L35
.L51:
	.loc 1 239 41
	movq	-120(%rbp), %rax
	.loc 1 239 12
	subq	$32, %rax
	movq	%rax, -80(%rbp)
	.loc 1 241 47
	movq	-80(%rbp), %rax
	shrq	$8, %rax
	.loc 1 241 28
	movl	%eax, %edx
	.loc 1 241 26
	movq	-72(%rbp), %rax
	movb	%dl, 603(%rax)
	.loc 1 242 28
	movq	-80(%rbp), %rax
	movl	%eax, %edx
	.loc 1 242 26
	movq	-72(%rbp), %rax
	movb	%dl, 604(%rax)
	.loc 1 251 39
	movq	-72(%rbp), %rax
	leaq	606(%rax), %r12
	.loc 1 250 44
	movq	-72(%rbp), %rax
	leaq	592(%rax), %rbx
	.loc 1 249 10
	call	aws_lc_0_38_0_EVP_sha256@PLT
	movq	%rax, %rdi
	.loc 1 249 10 is_stmt 0 discriminator 1
	movq	-80(%rbp), %rsi
	movq	-288(%rbp), %rcx
	leaq	-136(%rbp), %rdx
	leaq	-272(%rbp), %rax
	subq	$8, %rsp
	pushq	$64
	pushq	%r12
	pushq	-304(%rbp)
	movq	%rsi, %r9
	movq	%rcx, %r8
	movq	%rbx, %rcx
	movq	%rax, %rsi
	call	aws_lc_0_38_0_EVP_tls_cbc_digest_record@PLT
	addq	$32, %rsp
	.loc 1 249 8 is_stmt 1 discriminator 2
	testl	%eax, %eax
	jne	.L52
	.loc 1 252 7
	movl	$252, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 253 14
	movl	$0, %eax
	jmp	.L35
.L52:
	.loc 1 255 5
	movq	-136(%rbp), %rax
	cmpq	$32, %rax
	je	.L53
	.loc 1 255 5 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$255, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC1(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L53:
	.loc 1 257 16 is_stmt 1
	leaq	-208(%rbp), %rax
	movq	%rax, -88(%rbp)
	.loc 1 258 5
	movq	-120(%rbp), %rcx
	movq	-136(%rbp), %rsi
	movq	-304(%rbp), %rdi
	movq	-288(%rbp), %rdx
	movq	-88(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_tls_cbc_copy_mac@PLT
	.loc 1 265 9
	movq	-136(%rbp), %rdx
	leaq	-272(%rbp), %rcx
	movq	-88(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_memcmp@PLT
	.loc 1 265 9 is_stmt 0 discriminator 1
	movl	$0, %esi
	movl	%eax, %edi
	call	constant_time_eq_int
	movq	%rax, -96(%rbp)
	.loc 1 266 10 is_stmt 1
	movq	-128(%rbp), %rax
	andq	%rax, -96(%rbp)
	.loc 1 268 8
	cmpq	$0, -96(%rbp)
	jne	.L54
	.loc 1 269 7
	movl	$269, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 271 14
	movl	$0, %eax
	jmp	.L35
.L54:
	.loc 1 278 12
	movl	$1, %eax
.L35:
.LBE2:
	.loc 1 280 1
	leaq	-24(%rbp), %rsp
	popq	%rbx
	popq	%r10
	popq	%r12
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE293:
	.size	aesni_cbc_hmac_sha256_cipher, .-aesni_cbc_hmac_sha256_cipher
	.section	.text.aesni_cbc_hmac_sha256_ctrl,"ax",@progbits
	.type	aesni_cbc_hmac_sha256_ctrl, @function
aesni_cbc_hmac_sha256_ctrl:
.LFB294:
	.loc 1 283 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%rbx
	subq	$152, %rsp
	.cfi_offset 3, -24
	movq	%rdi, 24(%rsp)
	movl	%esi, 20(%rsp)
	movl	%edx, 16(%rsp)
	movq	%rcx, 8(%rsp)
	.loc 1 284 24
	movq	24(%rsp), %rax
	movq	16(%rax), %rax
	movq	%rax, 112(%rsp)
	.loc 1 286 3
	cmpl	$22, 20(%rsp)
	je	.L56
	cmpl	$23, 20(%rsp)
	jne	.L57
.LBB3:
	.loc 1 288 10
	cmpl	$0, 16(%rsp)
	jns	.L58
	.loc 1 289 16
	movl	$0, %eax
	jmp	.L66
.L58:
	.loc 1 292 7
	leaq	32(%rsp), %rax
	movl	$64, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 293 14
	movl	16(%rsp), %eax
	cltq
	movq	%rax, 104(%rsp)
	.loc 1 294 10
	cmpq	$64, 104(%rsp)
	jbe	.L60
	.loc 1 295 9
	movq	112(%rsp), %rax
	addq	$244, %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Init@PLT
	.loc 1 296 9
	movl	16(%rsp), %eax
	movslq	%eax, %rdx
	movq	112(%rsp), %rax
	leaq	244(%rax), %rcx
	movq	8(%rsp), %rax
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 297 9
	movq	112(%rsp), %rax
	leaq	244(%rax), %rdx
	leaq	32(%rsp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Final@PLT
	jmp	.L61
.L60:
	.loc 1 299 9
	movl	16(%rsp), %eax
	movslq	%eax, %rdx
	movq	8(%rsp), %rcx
	leaq	32(%rsp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
.L61:
	.loc 1 301 22
	movq	112(%rsp), %rax
	leaq	606(%rax), %rcx
	.loc 1 301 7
	leaq	32(%rsp), %rax
	movl	$64, %edx
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
.LBB4:
	.loc 1 303 19
	movq	$0, 136(%rsp)
	.loc 1 303 7
	jmp	.L62
.L63:
	.loc 1 304 17
	leaq	32(%rsp), %rdx
	movq	136(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	.loc 1 304 21
	xorl	$54, %eax
	movl	%eax, %edx
	leaq	32(%rsp), %rcx
	movq	136(%rsp), %rax
	addq	%rcx, %rax
	movb	%dl, (%rax)
	.loc 1 303 49 discriminator 3
	addq	$1, 136(%rsp)
.L62:
	.loc 1 303 28 discriminator 1
	cmpq	$63, 136(%rsp)
	jbe	.L63
.LBE4:
	.loc 1 306 7
	movq	112(%rsp), %rax
	addq	$244, %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Init@PLT
	.loc 1 307 7
	movq	112(%rsp), %rax
	leaq	244(%rax), %rcx
	leaq	32(%rsp), %rax
	movl	$64, %edx
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
.LBB5:
	.loc 1 309 19
	movq	$0, 128(%rsp)
	.loc 1 309 7
	jmp	.L64
.L65:
	.loc 1 310 17
	leaq	32(%rsp), %rdx
	movq	128(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	.loc 1 310 21
	xorl	$106, %eax
	movl	%eax, %edx
	leaq	32(%rsp), %rcx
	movq	128(%rsp), %rax
	addq	%rcx, %rax
	movb	%dl, (%rax)
	.loc 1 309 49 discriminator 3
	addq	$1, 128(%rsp)
.L64:
	.loc 1 309 28 discriminator 1
	cmpq	$63, 128(%rsp)
	jbe	.L65
.LBE5:
	.loc 1 312 7
	movq	112(%rsp), %rax
	addq	$356, %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Init@PLT
	.loc 1 313 7
	movq	112(%rsp), %rax
	leaq	356(%rax), %rcx
	leaq	32(%rsp), %rax
	movl	$64, %edx
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 315 7
	leaq	32(%rsp), %rax
	movl	$64, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_cleanse@PLT
	.loc 1 317 14
	movl	$1, %eax
	jmp	.L66
.L56:
.LBE3:
.LBB6:
	.loc 1 320 10
	cmpl	$13, 16(%rsp)
	je	.L67
	.loc 1 321 9
	movl	$321, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$109, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 322 16
	movl	$0, %eax
	jmp	.L66
.L67:
	.loc 1 330 16
	movq	8(%rsp), %rax
	movq	%rax, 96(%rsp)
	.loc 1 331 23
	movl	16(%rsp), %eax
	cltq
	leaq	-2(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	.loc 1 331 38
	movzbl	%al, %eax
	sall	$8, %eax
	movl	%eax, %ecx
	.loc 1 331 41
	movl	16(%rsp), %eax
	cltq
	leaq	-1(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	movzbl	%al, %eax
	.loc 1 331 38
	orl	%ecx, %eax
	.loc 1 331 16
	movw	%ax, 126(%rsp)
	.loc 1 333 11
	movq	24(%rsp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_CIPHER_CTX_encrypting@PLT
	.loc 1 333 10 discriminator 1
	testl	%eax, %eax
	je	.L68
	.loc 1 334 29
	movzwl	126(%rsp), %edx
	movq	112(%rsp), %rax
	movq	%rdx, 584(%rax)
	.loc 1 335 34
	movl	16(%rsp), %eax
	cltq
	leaq	-4(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	.loc 1 335 49
	movzbl	%al, %eax
	sall	$8, %eax
	movl	%eax, %ecx
	.loc 1 335 52
	movl	16(%rsp), %eax
	cltq
	leaq	-3(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	movzbl	%al, %eax
	.loc 1 335 49
	orl	%ecx, %eax
	movl	%eax, %edx
	.loc 1 335 31
	movq	112(%rsp), %rax
	movw	%dx, 592(%rax)
	.loc 1 335 22
	movq	112(%rsp), %rax
	movzwl	592(%rax), %eax
	.loc 1 335 12
	cmpw	$769, %ax
	jbe	.L69
	.loc 1 337 14
	cmpw	$15, 126(%rsp)
	ja	.L70
	.loc 1 338 13
	movl	$338, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$109, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 339 20
	movl	$0, %eax
	jmp	.L66
.L70:
	.loc 1 341 15
	subw	$16, 126(%rsp)
	.loc 1 342 22
	movzwl	126(%rsp), %eax
	shrw	$8, %ax
	movl	%eax, %ecx
	.loc 1 342 12
	movl	16(%rsp), %eax
	cltq
	leaq	-2(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	.loc 1 342 22
	movl	%ecx, %edx
	movb	%dl, (%rax)
	.loc 1 343 12
	movl	16(%rsp), %eax
	cltq
	leaq	-1(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	.loc 1 343 22
	movzwl	126(%rsp), %edx
	movb	%dl, (%rax)
.L69:
	.loc 1 345 17
	movq	112(%rsp), %rax
	movq	112(%rsp), %rdx
	vmovdqu	244(%rdx), %ymm0
	vmovdqu	%ymm0, 468(%rax)
	vmovdqu	276(%rdx), %ymm0
	vmovdqu	%ymm0, 500(%rax)
	vmovdqu	308(%rdx), %ymm0
	vmovdqu	%ymm0, 532(%rax)
	movq	340(%rdx), %rcx
	movq	348(%rdx), %rbx
	movq	%rcx, 564(%rax)
	movq	%rbx, 572(%rax)
	.loc 1 346 9
	movl	16(%rsp), %eax
	movslq	%eax, %rdx
	movq	112(%rsp), %rax
	leaq	468(%rax), %rcx
	movq	96(%rsp), %rax
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 348 51
	movzwl	126(%rsp), %eax
	addl	$48, %eax
	.loc 1 348 69
	andl	$-16, %eax
	.loc 1 348 16
	movzwl	126(%rsp), %edx
	subl	%edx, %eax
	jmp	.L66
.L68:
	.loc 1 352 9
	movl	16(%rsp), %eax
	movslq	%eax, %rdx
	.loc 1 352 32
	movq	112(%rsp), %rax
	leaq	592(%rax), %rcx
	.loc 1 352 9
	movq	8(%rsp), %rax
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
	.loc 1 353 29
	movl	16(%rsp), %eax
	movslq	%eax, %rdx
	movq	112(%rsp), %rax
	movq	%rdx, 584(%rax)
	.loc 1 355 16
	movl	$32, %eax
	jmp	.L66
.L57:
.LBE6:
	.loc 1 359 7
	movl	$359, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$104, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 360 14
	movl	$0, %eax
.L66:
	.loc 1 362 1
	movq	-8(%rbp), %rbx
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE294:
	.size	aesni_cbc_hmac_sha256_ctrl, .-aesni_cbc_hmac_sha256_ctrl
	.section	.data.rel.ro.local.aesni_128_cbc_hmac_sha256_cipher,"aw"
	.align 32
	.type	aesni_128_cbc_hmac_sha256_cipher, @object
	.size	aesni_128_cbc_hmac_sha256_cipher, 56
aesni_128_cbc_hmac_sha256_cipher:
	.long	963
	.long	16
	.long	16
	.long	16
	.long	672
	.long	2050
	.quad	aesni_cbc_hmac_sha256_init_key
	.quad	aesni_cbc_hmac_sha256_cipher
	.quad	0
	.quad	aesni_cbc_hmac_sha256_ctrl
	.section	.data.rel.ro.local.aesni_256_cbc_hmac_sha256_cipher,"aw"
	.align 32
	.type	aesni_256_cbc_hmac_sha256_cipher, @object
	.size	aesni_256_cbc_hmac_sha256_cipher, 56
aesni_256_cbc_hmac_sha256_cipher:
	.long	964
	.long	16
	.long	32
	.long	16
	.long	672
	.long	2050
	.quad	aesni_cbc_hmac_sha256_init_key
	.quad	aesni_cbc_hmac_sha256_cipher
	.quad	0
	.quad	aesni_cbc_hmac_sha256_ctrl
	.section	.text.aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha256,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha256
	.type	aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha256, @function
aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha256:
.LFB295:
	.loc 1 388 53
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 1 389 10
	call	hwaes_capable
	.loc 1 389 62 discriminator 1
	testl	%eax, %eax
	je	.L72
	leaq	aesni_128_cbc_hmac_sha256_cipher(%rip), %rax
	.loc 1 389 62 is_stmt 0
	jmp	.L74
.L72:
	.loc 1 389 62 discriminator 2
	movl	$0, %eax
.L74:
	.loc 1 390 1 is_stmt 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE295:
	.size	aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha256, .-aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha256
	.section	.text.aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha256,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha256
	.type	aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha256, @function
aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha256:
.LFB296:
	.loc 1 392 53
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 1 393 10
	call	hwaes_capable
	.loc 1 393 62 discriminator 1
	testl	%eax, %eax
	je	.L76
	leaq	aesni_256_cbc_hmac_sha256_cipher(%rip), %rax
	.loc 1 393 62 is_stmt 0
	jmp	.L78
.L76:
	.loc 1 393 62 discriminator 2
	movl	$0, %eax
.L78:
	.loc 1 394 1 is_stmt 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE296:
	.size	aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha256, .-aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha256
	.section	.rodata.__PRETTY_FUNCTION__.0,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.0, @object
	.size	__PRETTY_FUNCTION__.0, 29
__PRETTY_FUNCTION__.0:
	.string	"aesni_cbc_hmac_sha256_cipher"
	.text
.Letext0:
	.file 5 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 6 "/usr/include/bits/types.h"
	.file 7 "/usr/include/bits/stdint-uintn.h"
	.file 8 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.file 9 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/cipher.h"
	.file 10 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/../fipsmodule/cipher/internal.h"
	.file 11 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/sha.h"
	.file 12 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/aes.h"
	.file 13 "/usr/include/string.h"
	.file 14 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/mem.h"
	.file 15 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/internal.h"
	.file 16 "/usr/include/assert.h"
	.file 17 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/err.h"
	.file 18 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/digest.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0xdbd
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF114
	.byte	0xc
	.long	.LASF115
	.long	.LASF116
	.long	.Ldebug_ranges0+0
	.quad	0
	.long	.Ldebug_line0
	.uleb128 0x2
	.long	.LASF5
	.byte	0x5
	.byte	0xe5
	.byte	0x17
	.long	0x35
	.uleb128 0x3
	.byte	0x8
	.byte	0x7
	.long	.LASF0
	.uleb128 0x3
	.byte	0x4
	.byte	0x7
	.long	.LASF1
	.uleb128 0x4
	.byte	0x8
	.uleb128 0x3
	.byte	0x1
	.byte	0x8
	.long	.LASF2
	.uleb128 0x3
	.byte	0x2
	.byte	0x7
	.long	.LASF3
	.uleb128 0x3
	.byte	0x1
	.byte	0x6
	.long	.LASF4
	.uleb128 0x2
	.long	.LASF6
	.byte	0x6
	.byte	0x26
	.byte	0x17
	.long	0x45
	.uleb128 0x3
	.byte	0x2
	.byte	0x5
	.long	.LASF7
	.uleb128 0x2
	.long	.LASF8
	.byte	0x6
	.byte	0x28
	.byte	0x1c
	.long	0x4c
	.uleb128 0x5
	.byte	0x4
	.byte	0x5
	.string	"int"
	.uleb128 0x6
	.long	0x79
	.uleb128 0x2
	.long	.LASF9
	.byte	0x6
	.byte	0x2a
	.byte	0x16
	.long	0x3c
	.uleb128 0x3
	.byte	0x8
	.byte	0x5
	.long	.LASF10
	.uleb128 0x2
	.long	.LASF11
	.byte	0x6
	.byte	0x2d
	.byte	0x1b
	.long	0x35
	.uleb128 0x3
	.byte	0x1
	.byte	0x6
	.long	.LASF12
	.uleb128 0x6
	.long	0xa4
	.uleb128 0x7
	.byte	0x8
	.long	0xab
	.uleb128 0x3
	.byte	0x8
	.byte	0x5
	.long	.LASF13
	.uleb128 0x3
	.byte	0x10
	.byte	0x4
	.long	.LASF14
	.uleb128 0x2
	.long	.LASF15
	.byte	0x7
	.byte	0x18
	.byte	0x13
	.long	0x5a
	.uleb128 0x6
	.long	0xc4
	.uleb128 0x2
	.long	.LASF16
	.byte	0x7
	.byte	0x19
	.byte	0x14
	.long	0x6d
	.uleb128 0x2
	.long	.LASF17
	.byte	0x7
	.byte	0x1a
	.byte	0x14
	.long	0x85
	.uleb128 0x6
	.long	0xe1
	.uleb128 0x2
	.long	.LASF18
	.byte	0x7
	.byte	0x1b
	.byte	0x14
	.long	0x98
	.uleb128 0x3
	.byte	0x8
	.byte	0x7
	.long	.LASF19
	.uleb128 0x7
	.byte	0x8
	.long	0x10b
	.uleb128 0x8
	.uleb128 0x9
	.long	.LASF20
	.byte	0x8
	.value	0x1a5
	.byte	0x1a
	.long	0x11e
	.uleb128 0x6
	.long	0x10c
	.uleb128 0xa
	.long	.LASF117
	.uleb128 0x9
	.long	.LASF21
	.byte	0x8
	.value	0x1a8
	.byte	0x22
	.long	0x135
	.uleb128 0x6
	.long	0x123
	.uleb128 0xb
	.long	.LASF33
	.byte	0x98
	.byte	0x9
	.value	0x272
	.byte	0x8
	.long	0x207
	.uleb128 0xc
	.long	.LASF22
	.byte	0x9
	.value	0x274
	.byte	0x15
	.long	0x357
	.byte	0
	.uleb128 0xc
	.long	.LASF23
	.byte	0x9
	.value	0x277
	.byte	0x9
	.long	0x43
	.byte	0x8
	.uleb128 0xc
	.long	.LASF24
	.byte	0x9
	.value	0x27a
	.byte	0x9
	.long	0x43
	.byte	0x10
	.uleb128 0xc
	.long	.LASF25
	.byte	0x9
	.value	0x27e
	.byte	0xc
	.long	0x3c
	.byte	0x18
	.uleb128 0xc
	.long	.LASF26
	.byte	0x9
	.value	0x281
	.byte	0x7
	.long	0x79
	.byte	0x1c
	.uleb128 0xc
	.long	.LASF27
	.byte	0x9
	.value	0x284
	.byte	0xc
	.long	0xe1
	.byte	0x20
	.uleb128 0xd
	.string	"oiv"
	.byte	0x9
	.value	0x287
	.byte	0xb
	.long	0x35d
	.byte	0x24
	.uleb128 0xd
	.string	"iv"
	.byte	0x9
	.value	0x28a
	.byte	0xb
	.long	0x35d
	.byte	0x34
	.uleb128 0xd
	.string	"buf"
	.byte	0x9
	.value	0x28e
	.byte	0xb
	.long	0x36d
	.byte	0x44
	.uleb128 0xc
	.long	.LASF28
	.byte	0x9
	.value	0x292
	.byte	0x7
	.long	0x79
	.byte	0x64
	.uleb128 0xd
	.string	"num"
	.byte	0x9
	.value	0x296
	.byte	0xc
	.long	0x3c
	.byte	0x68
	.uleb128 0xc
	.long	.LASF29
	.byte	0x9
	.value	0x299
	.byte	0x7
	.long	0x79
	.byte	0x6c
	.uleb128 0xc
	.long	.LASF30
	.byte	0x9
	.value	0x29b
	.byte	0xb
	.long	0x36d
	.byte	0x70
	.uleb128 0xc
	.long	.LASF31
	.byte	0x9
	.value	0x29e
	.byte	0x7
	.long	0x79
	.byte	0x90
	.byte	0
	.uleb128 0x9
	.long	.LASF32
	.byte	0x8
	.value	0x1a9
	.byte	0x1e
	.long	0x219
	.uleb128 0x6
	.long	0x207
	.uleb128 0xe
	.long	.LASF34
	.byte	0x38
	.byte	0xa
	.byte	0x9c
	.byte	0x8
	.long	0x2a9
	.uleb128 0xf
	.string	"nid"
	.byte	0xa
	.byte	0x9e
	.byte	0x7
	.long	0x79
	.byte	0
	.uleb128 0x10
	.long	.LASF35
	.byte	0xa
	.byte	0xa2
	.byte	0xc
	.long	0x3c
	.byte	0x4
	.uleb128 0x10
	.long	.LASF25
	.byte	0xa
	.byte	0xa6
	.byte	0xc
	.long	0x3c
	.byte	0x8
	.uleb128 0x10
	.long	.LASF36
	.byte	0xa
	.byte	0xa9
	.byte	0xc
	.long	0x3c
	.byte	0xc
	.uleb128 0x10
	.long	.LASF37
	.byte	0xa
	.byte	0xad
	.byte	0xc
	.long	0x3c
	.byte	0x10
	.uleb128 0x10
	.long	.LASF27
	.byte	0xa
	.byte	0xb0
	.byte	0xc
	.long	0xe1
	.byte	0x14
	.uleb128 0x10
	.long	.LASF38
	.byte	0xa
	.byte	0xb2
	.byte	0x9
	.long	0x42a
	.byte	0x18
	.uleb128 0x10
	.long	.LASF22
	.byte	0xa
	.byte	0xb5
	.byte	0x9
	.long	0x44e
	.byte	0x20
	.uleb128 0x10
	.long	.LASF39
	.byte	0xa
	.byte	0xbb
	.byte	0xa
	.long	0x45f
	.byte	0x28
	.uleb128 0x10
	.long	.LASF40
	.byte	0xa
	.byte	0xbd
	.byte	0x9
	.long	0x483
	.byte	0x30
	.byte	0
	.uleb128 0x9
	.long	.LASF41
	.byte	0x8
	.value	0x1d5
	.byte	0x20
	.long	0x2b6
	.uleb128 0xe
	.long	.LASF42
	.byte	0x70
	.byte	0xb
	.byte	0xae
	.byte	0x8
	.long	0x30e
	.uleb128 0xf
	.string	"h"
	.byte	0xb
	.byte	0xaf
	.byte	0xc
	.long	0x38d
	.byte	0
	.uleb128 0xf
	.string	"Nl"
	.byte	0xb
	.byte	0xb0
	.byte	0xc
	.long	0xe1
	.byte	0x20
	.uleb128 0xf
	.string	"Nh"
	.byte	0xb
	.byte	0xb0
	.byte	0x10
	.long	0xe1
	.byte	0x24
	.uleb128 0x10
	.long	.LASF43
	.byte	0xb
	.byte	0xb1
	.byte	0xb
	.long	0x37d
	.byte	0x28
	.uleb128 0xf
	.string	"num"
	.byte	0xb
	.byte	0xb2
	.byte	0xc
	.long	0x3c
	.byte	0x68
	.uleb128 0x10
	.long	.LASF44
	.byte	0xb
	.byte	0xb2
	.byte	0x11
	.long	0x3c
	.byte	0x6c
	.byte	0
	.uleb128 0xe
	.long	.LASF45
	.byte	0xf4
	.byte	0xc
	.byte	0x48
	.byte	0x8
	.long	0x336
	.uleb128 0x10
	.long	.LASF46
	.byte	0xc
	.byte	0x49
	.byte	0xc
	.long	0x336
	.byte	0
	.uleb128 0x10
	.long	.LASF47
	.byte	0xc
	.byte	0x4a
	.byte	0xc
	.long	0x3c
	.byte	0xf0
	.byte	0
	.uleb128 0x11
	.long	0xe1
	.long	0x346
	.uleb128 0x12
	.long	0x35
	.byte	0x3b
	.byte	0
	.uleb128 0x2
	.long	.LASF48
	.byte	0xc
	.byte	0x4c
	.byte	0x1b
	.long	0x30e
	.uleb128 0x6
	.long	0x346
	.uleb128 0x7
	.byte	0x8
	.long	0x214
	.uleb128 0x11
	.long	0xc4
	.long	0x36d
	.uleb128 0x12
	.long	0x35
	.byte	0xf
	.byte	0
	.uleb128 0x11
	.long	0xc4
	.long	0x37d
	.uleb128 0x12
	.long	0x35
	.byte	0x1f
	.byte	0
	.uleb128 0x11
	.long	0xc4
	.long	0x38d
	.uleb128 0x12
	.long	0x35
	.byte	0x3f
	.byte	0
	.uleb128 0x11
	.long	0xe1
	.long	0x39d
	.uleb128 0x12
	.long	0x35
	.byte	0x7
	.byte	0
	.uleb128 0x3
	.byte	0x8
	.byte	0x4
	.long	.LASF49
	.uleb128 0x7
	.byte	0x8
	.long	0x119
	.uleb128 0x7
	.byte	0x8
	.long	0xd0
	.uleb128 0x7
	.byte	0x8
	.long	0xc4
	.uleb128 0x7
	.byte	0x8
	.long	0x29
	.uleb128 0x11
	.long	0xe1
	.long	0x3cc
	.uleb128 0x12
	.long	0x35
	.byte	0x3
	.byte	0
	.uleb128 0x13
	.long	.LASF118
	.byte	0x2
	.byte	0x28
	.byte	0x11
	.long	0x3bc
	.uleb128 0x3
	.byte	0x10
	.byte	0x5
	.long	.LASF50
	.uleb128 0x3
	.byte	0x10
	.byte	0x7
	.long	.LASF51
	.uleb128 0x9
	.long	.LASF52
	.byte	0x3
	.value	0x12b
	.byte	0x12
	.long	0xf2
	.uleb128 0x3
	.byte	0x1
	.byte	0x2
	.long	.LASF53
	.uleb128 0x7
	.byte	0x8
	.long	0x352
	.uleb128 0x7
	.byte	0x8
	.long	0x346
	.uleb128 0x14
	.long	0x79
	.long	0x424
	.uleb128 0x15
	.long	0x424
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x79
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x123
	.uleb128 0x7
	.byte	0x8
	.long	0x406
	.uleb128 0x14
	.long	0x79
	.long	0x44e
	.uleb128 0x15
	.long	0x424
	.uleb128 0x15
	.long	0x3b0
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x430
	.uleb128 0x16
	.long	0x45f
	.uleb128 0x15
	.long	0x424
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x454
	.uleb128 0x14
	.long	0x79
	.long	0x483
	.uleb128 0x15
	.long	0x424
	.uleb128 0x15
	.long	0x79
	.uleb128 0x15
	.long	0x79
	.uleb128 0x15
	.long	0x43
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x465
	.uleb128 0x17
	.byte	0xe
	.byte	0x1
	.byte	0x27
	.byte	0x3
	.long	0x4ab
	.uleb128 0x18
	.long	.LASF54
	.byte	0x1
	.byte	0x28
	.byte	0xe
	.long	0xd5
	.uleb128 0x18
	.long	.LASF55
	.byte	0x1
	.byte	0x33
	.byte	0xd
	.long	0x4ab
	.byte	0
	.uleb128 0x11
	.long	0xc4
	.long	0x4bb
	.uleb128 0x12
	.long	0x35
	.byte	0xc
	.byte	0
	.uleb128 0x19
	.value	0x2a0
	.byte	0x1
	.byte	0x1d
	.byte	0x9
	.long	0x524
	.uleb128 0xf
	.string	"ks"
	.byte	0x1
	.byte	0x1e
	.byte	0xb
	.long	0x346
	.byte	0
	.uleb128 0x10
	.long	.LASF56
	.byte	0x1
	.byte	0x23
	.byte	0xe
	.long	0x2a9
	.byte	0xf4
	.uleb128 0x1a
	.long	.LASF57
	.byte	0x1
	.byte	0x23
	.byte	0x14
	.long	0x2a9
	.value	0x164
	.uleb128 0x1b
	.string	"md"
	.byte	0x1
	.byte	0x23
	.byte	0x1a
	.long	0x2a9
	.value	0x1d4
	.uleb128 0x1a
	.long	.LASF58
	.byte	0x1
	.byte	0x26
	.byte	0xa
	.long	0x29
	.value	0x248
	.uleb128 0x1b
	.string	"aux"
	.byte	0x1
	.byte	0x34
	.byte	0x5
	.long	0x489
	.value	0x250
	.uleb128 0x1a
	.long	.LASF59
	.byte	0x1
	.byte	0x36
	.byte	0xb
	.long	0x37d
	.value	0x25e
	.byte	0
	.uleb128 0x2
	.long	.LASF60
	.byte	0x1
	.byte	0x37
	.byte	0x3
	.long	0x4bb
	.uleb128 0x1c
	.long	.LASF61
	.byte	0x1
	.value	0x16c
	.byte	0x19
	.long	0x214
	.uleb128 0x9
	.byte	0x3
	.quad	aesni_128_cbc_hmac_sha256_cipher
	.uleb128 0x1c
	.long	.LASF62
	.byte	0x1
	.value	0x178
	.byte	0x19
	.long	0x214
	.uleb128 0x9
	.byte	0x3
	.quad	aesni_256_cbc_hmac_sha256_cipher
	.uleb128 0x1d
	.long	.LASF63
	.byte	0xd
	.byte	0x3d
	.byte	0xe
	.long	0x43
	.long	0x57e
	.uleb128 0x15
	.long	0x43
	.uleb128 0x15
	.long	0x79
	.uleb128 0x15
	.long	0x35
	.byte	0
	.uleb128 0x1e
	.long	.LASF66
	.byte	0xe
	.byte	0x6d
	.byte	0x15
	.long	0x595
	.uleb128 0x15
	.long	0x43
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x1d
	.long	.LASF64
	.byte	0xd
	.byte	0x2b
	.byte	0xe
	.long	0x43
	.long	0x5b5
	.uleb128 0x15
	.long	0x43
	.uleb128 0x15
	.long	0x105
	.uleb128 0x15
	.long	0x35
	.byte	0
	.uleb128 0x1d
	.long	.LASF65
	.byte	0xe
	.byte	0x74
	.byte	0x14
	.long	0x79
	.long	0x5d5
	.uleb128 0x15
	.long	0x105
	.uleb128 0x15
	.long	0x105
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x1e
	.long	.LASF67
	.byte	0xf
	.byte	0x6a
	.byte	0x6
	.long	0x5fb
	.uleb128 0x15
	.long	0x3b0
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x1f
	.long	.LASF68
	.byte	0x10
	.byte	0x43
	.byte	0xd
	.long	0x61c
	.uleb128 0x15
	.long	0xb0
	.uleb128 0x15
	.long	0xb0
	.uleb128 0x15
	.long	0x3c
	.uleb128 0x15
	.long	0xb0
	.byte	0
	.uleb128 0x1d
	.long	.LASF69
	.byte	0xf
	.byte	0x97
	.byte	0x5
	.long	0x79
	.long	0x65a
	.uleb128 0x15
	.long	0x3a4
	.uleb128 0x15
	.long	0x3b0
	.uleb128 0x15
	.long	0x3b6
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x3c
	.byte	0
	.uleb128 0x20
	.long	.LASF119
	.byte	0x12
	.byte	0x55
	.byte	0x1e
	.long	0x3a4
	.uleb128 0x1d
	.long	.LASF70
	.byte	0xf
	.byte	0x5e
	.byte	0x5
	.long	0x79
	.long	0x695
	.uleb128 0x15
	.long	0x695
	.uleb128 0x15
	.long	0x3b6
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x3e6
	.uleb128 0x1e
	.long	.LASF71
	.byte	0x4
	.byte	0x66
	.byte	0x6
	.long	0x6c6
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x3b0
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x3fa
	.uleb128 0x15
	.long	0x3b0
	.uleb128 0x15
	.long	0x80
	.byte	0
	.uleb128 0x1d
	.long	.LASF72
	.byte	0xb
	.byte	0x97
	.byte	0x14
	.long	0x79
	.long	0x6e1
	.uleb128 0x15
	.long	0x3b0
	.uleb128 0x15
	.long	0x6e1
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x2a9
	.uleb128 0x1e
	.long	.LASF73
	.byte	0x1
	.byte	0x39
	.byte	0x6
	.long	0x717
	.uleb128 0x15
	.long	0x105
	.uleb128 0x15
	.long	0x43
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x3fa
	.uleb128 0x15
	.long	0x3b0
	.uleb128 0x15
	.long	0x6e1
	.uleb128 0x15
	.long	0x105
	.byte	0
	.uleb128 0x1d
	.long	.LASF74
	.byte	0xb
	.byte	0x92
	.byte	0x14
	.long	0x79
	.long	0x737
	.uleb128 0x15
	.long	0x6e1
	.uleb128 0x15
	.long	0x105
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x21
	.long	.LASF75
	.byte	0x9
	.value	0x100
	.byte	0x14
	.long	0x79
	.long	0x74e
	.uleb128 0x15
	.long	0x74e
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x130
	.uleb128 0x22
	.long	.LASF76
	.byte	0x11
	.value	0x1d0
	.byte	0x15
	.long	0x77b
	.uleb128 0x15
	.long	0x79
	.uleb128 0x15
	.long	0x79
	.uleb128 0x15
	.long	0x79
	.uleb128 0x15
	.long	0xb0
	.uleb128 0x15
	.long	0x3c
	.byte	0
	.uleb128 0x1d
	.long	.LASF77
	.byte	0xb
	.byte	0x8f
	.byte	0x14
	.long	0x79
	.long	0x791
	.uleb128 0x15
	.long	0x6e1
	.byte	0
	.uleb128 0x1d
	.long	.LASF78
	.byte	0x4
	.byte	0x62
	.byte	0x5
	.long	0x79
	.long	0x7b1
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x80
	.uleb128 0x15
	.long	0x400
	.byte	0
	.uleb128 0x1d
	.long	.LASF79
	.byte	0x4
	.byte	0x60
	.byte	0x5
	.long	0x79
	.long	0x7d1
	.uleb128 0x15
	.long	0x3aa
	.uleb128 0x15
	.long	0x80
	.uleb128 0x15
	.long	0x400
	.byte	0
	.uleb128 0x21
	.long	.LASF80
	.byte	0x9
	.value	0x109
	.byte	0x19
	.long	0x3c
	.long	0x7e8
	.uleb128 0x15
	.long	0x74e
	.byte	0
	.uleb128 0x23
	.long	.LASF81
	.byte	0x1
	.value	0x188
	.byte	0x13
	.long	0x357
	.quad	.LFB296
	.quad	.LFE296-.LFB296
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x23
	.long	.LASF82
	.byte	0x1
	.value	0x184
	.byte	0x13
	.long	0x357
	.quad	.LFB295
	.quad	.LFE295-.LFB295
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x24
	.long	.LASF85
	.byte	0x1
	.value	0x11a
	.byte	0xc
	.long	0x79
	.quad	.LFB294
	.quad	.LFE294-.LFB294
	.uleb128 0x1
	.byte	0x9c
	.long	0x94a
	.uleb128 0x25
	.string	"ctx"
	.byte	0x1
	.value	0x11a
	.byte	0x37
	.long	0x424
	.uleb128 0x2
	.byte	0x77
	.sleb128 24
	.uleb128 0x26
	.long	.LASF83
	.byte	0x1
	.value	0x11a
	.byte	0x40
	.long	0x79
	.uleb128 0x2
	.byte	0x77
	.sleb128 20
	.uleb128 0x25
	.string	"arg"
	.byte	0x1
	.value	0x11a
	.byte	0x4a
	.long	0x79
	.uleb128 0x2
	.byte	0x77
	.sleb128 16
	.uleb128 0x25
	.string	"ptr"
	.byte	0x1
	.value	0x11b
	.byte	0x2d
	.long	0x43
	.uleb128 0x2
	.byte	0x77
	.sleb128 8
	.uleb128 0x27
	.string	"key"
	.byte	0x1
	.value	0x11c
	.byte	0x18
	.long	0x94a
	.uleb128 0x3
	.byte	0x77
	.sleb128 112
	.uleb128 0x28
	.quad	.LBB3
	.quad	.LBE3-.LBB3
	.long	0x917
	.uleb128 0x1c
	.long	.LASF59
	.byte	0x1
	.value	0x123
	.byte	0xf
	.long	0x37d
	.uleb128 0x2
	.byte	0x77
	.sleb128 32
	.uleb128 0x1c
	.long	.LASF84
	.byte	0x1
	.value	0x125
	.byte	0xe
	.long	0x29
	.uleb128 0x3
	.byte	0x77
	.sleb128 104
	.uleb128 0x28
	.quad	.LBB4
	.quad	.LBE4-.LBB4
	.long	0x8f5
	.uleb128 0x27
	.string	"i"
	.byte	0x1
	.value	0x12f
	.byte	0x13
	.long	0x29
	.uleb128 0x3
	.byte	0x77
	.sleb128 136
	.byte	0
	.uleb128 0x29
	.quad	.LBB5
	.quad	.LBE5-.LBB5
	.uleb128 0x27
	.string	"i"
	.byte	0x1
	.value	0x135
	.byte	0x13
	.long	0x29
	.uleb128 0x3
	.byte	0x77
	.sleb128 128
	.byte	0
	.byte	0
	.uleb128 0x29
	.quad	.LBB6
	.quad	.LBE6-.LBB6
	.uleb128 0x27
	.string	"p"
	.byte	0x1
	.value	0x14a
	.byte	0x10
	.long	0x3b0
	.uleb128 0x3
	.byte	0x77
	.sleb128 96
	.uleb128 0x27
	.string	"len"
	.byte	0x1
	.value	0x14b
	.byte	0x10
	.long	0xd5
	.uleb128 0x3
	.byte	0x77
	.sleb128 126
	.byte	0
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x524
	.uleb128 0x2a
	.long	.LASF86
	.byte	0x1
	.byte	0x67
	.byte	0xc
	.long	0x79
	.quad	.LFB293
	.quad	.LFE293-.LFB293
	.uleb128 0x1
	.byte	0x9c
	.long	0xac2
	.uleb128 0x2b
	.string	"ctx"
	.byte	0x1
	.byte	0x67
	.byte	0x39
	.long	0x424
	.uleb128 0x3
	.byte	0x76
	.sleb128 -280
	.uleb128 0x2b
	.string	"out"
	.byte	0x1
	.byte	0x67
	.byte	0x47
	.long	0x3b0
	.uleb128 0x3
	.byte	0x76
	.sleb128 -288
	.uleb128 0x2b
	.string	"in"
	.byte	0x1
	.byte	0x68
	.byte	0x38
	.long	0x3aa
	.uleb128 0x3
	.byte	0x76
	.sleb128 -296
	.uleb128 0x2b
	.string	"len"
	.byte	0x1
	.byte	0x68
	.byte	0x43
	.long	0x29
	.uleb128 0x3
	.byte	0x76
	.sleb128 -304
	.uleb128 0x2c
	.string	"key"
	.byte	0x1
	.byte	0x69
	.byte	0x18
	.long	0x94a
	.uleb128 0x3
	.byte	0x76
	.sleb128 -72
	.uleb128 0x2c
	.string	"l"
	.byte	0x1
	.byte	0x6a
	.byte	0x10
	.long	0x3c
	.uleb128 0x3
	.byte	0x76
	.sleb128 -108
	.uleb128 0x2d
	.long	.LASF87
	.byte	0x1
	.byte	0x6b
	.byte	0xa
	.long	0x29
	.uleb128 0x2
	.byte	0x76
	.sleb128 -40
	.uleb128 0x2d
	.long	.LASF36
	.byte	0x1
	.byte	0x6b
	.byte	0x26
	.long	0x29
	.uleb128 0x2
	.byte	0x76
	.sleb128 -48
	.uleb128 0x2d
	.long	.LASF88
	.byte	0x1
	.byte	0x6c
	.byte	0xa
	.long	0x29
	.uleb128 0x2
	.byte	0x76
	.sleb128 -56
	.uleb128 0x2d
	.long	.LASF89
	.byte	0x1
	.byte	0x6d
	.byte	0xa
	.long	0x29
	.uleb128 0x3
	.byte	0x76
	.sleb128 -104
	.uleb128 0x2d
	.long	.LASF90
	.byte	0x1
	.byte	0x6e
	.byte	0xa
	.long	0x29
	.uleb128 0x2
	.byte	0x76
	.sleb128 -64
	.uleb128 0x2e
	.long	.LASF120
	.long	0xad2
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.0
	.uleb128 0x29
	.quad	.LBB2
	.quad	.LBE2-.LBB2
	.uleb128 0x2d
	.long	.LASF91
	.byte	0x1
	.byte	0xe6
	.byte	0xc
	.long	0x29
	.uleb128 0x3
	.byte	0x76
	.sleb128 -120
	.uleb128 0x2d
	.long	.LASF92
	.byte	0x1
	.byte	0xe7
	.byte	0x13
	.long	0x3e6
	.uleb128 0x3
	.byte	0x76
	.sleb128 -128
	.uleb128 0x2d
	.long	.LASF93
	.byte	0x1
	.byte	0xef
	.byte	0xc
	.long	0x29
	.uleb128 0x3
	.byte	0x76
	.sleb128 -80
	.uleb128 0x2c
	.string	"mac"
	.byte	0x1
	.byte	0xf5
	.byte	0xd
	.long	0x37d
	.uleb128 0x3
	.byte	0x76
	.sleb128 -272
	.uleb128 0x2d
	.long	.LASF94
	.byte	0x1
	.byte	0xf6
	.byte	0xc
	.long	0x29
	.uleb128 0x3
	.byte	0x76
	.sleb128 -136
	.uleb128 0x2d
	.long	.LASF95
	.byte	0x1
	.byte	0xf7
	.byte	0xd
	.long	0x37d
	.uleb128 0x3
	.byte	0x76
	.sleb128 -208
	.uleb128 0x2d
	.long	.LASF96
	.byte	0x1
	.byte	0xf8
	.byte	0xe
	.long	0x3b0
	.uleb128 0x3
	.byte	0x76
	.sleb128 -88
	.uleb128 0x1c
	.long	.LASF97
	.byte	0x1
	.value	0x108
	.byte	0x13
	.long	0x3e6
	.uleb128 0x3
	.byte	0x76
	.sleb128 -96
	.byte	0
	.byte	0
	.uleb128 0x11
	.long	0xab
	.long	0xad2
	.uleb128 0x12
	.long	0x35
	.byte	0x1c
	.byte	0
	.uleb128 0x6
	.long	0xac2
	.uleb128 0x2a
	.long	.LASF98
	.byte	0x1
	.byte	0x3e
	.byte	0xc
	.long	0x79
	.quad	.LFB292
	.quad	.LFE292-.LFB292
	.uleb128 0x1
	.byte	0x9c
	.long	0xb62
	.uleb128 0x2b
	.string	"ctx"
	.byte	0x1
	.byte	0x3e
	.byte	0x3b
	.long	0x424
	.uleb128 0x2
	.byte	0x77
	.sleb128 24
	.uleb128 0x2f
	.long	.LASF99
	.byte	0x1
	.byte	0x3f
	.byte	0x3a
	.long	0x3aa
	.uleb128 0x2
	.byte	0x77
	.sleb128 16
	.uleb128 0x2b
	.string	"iv"
	.byte	0x1
	.byte	0x40
	.byte	0x3a
	.long	0x3aa
	.uleb128 0x2
	.byte	0x77
	.sleb128 8
	.uleb128 0x2b
	.string	"enc"
	.byte	0x1
	.byte	0x40
	.byte	0x42
	.long	0x79
	.uleb128 0x2
	.byte	0x77
	.sleb128 4
	.uleb128 0x2c
	.string	"key"
	.byte	0x1
	.byte	0x41
	.byte	0x18
	.long	0x94a
	.uleb128 0x2
	.byte	0x77
	.sleb128 48
	.uleb128 0x2c
	.string	"ret"
	.byte	0x1
	.byte	0x42
	.byte	0x7
	.long	0x79
	.uleb128 0x2
	.byte	0x77
	.sleb128 60
	.uleb128 0x2d
	.long	.LASF100
	.byte	0x1
	.byte	0x44
	.byte	0x7
	.long	0x79
	.uleb128 0x2
	.byte	0x77
	.sleb128 44
	.byte	0
	.uleb128 0x30
	.long	.LASF107
	.byte	0x4
	.byte	0x2d
	.byte	0x14
	.long	0x79
	.quad	.LFB282
	.quad	.LFE282-.LFB282
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x24
	.long	.LASF101
	.byte	0x3
	.value	0x3cc
	.byte	0x15
	.long	0x43
	.quad	.LFB249
	.quad	.LFE249-.LFB249
	.uleb128 0x1
	.byte	0x9c
	.long	0xbd0
	.uleb128 0x25
	.string	"dst"
	.byte	0x3
	.value	0x3cc
	.byte	0x2a
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x25
	.string	"c"
	.byte	0x3
	.value	0x3cc
	.byte	0x33
	.long	0x79
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.uleb128 0x25
	.string	"n"
	.byte	0x3
	.value	0x3cc
	.byte	0x3d
	.long	0x29
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x24
	.long	.LASF102
	.byte	0x3
	.value	0x3bc
	.byte	0x15
	.long	0x43
	.quad	.LFB247
	.quad	.LFE247-.LFB247
	.uleb128 0x1
	.byte	0x9c
	.long	0xc22
	.uleb128 0x25
	.string	"dst"
	.byte	0x3
	.value	0x3bc
	.byte	0x2a
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x25
	.string	"src"
	.byte	0x3
	.value	0x3bc
	.byte	0x3b
	.long	0x105
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x25
	.string	"n"
	.byte	0x3
	.value	0x3bc
	.byte	0x47
	.long	0x29
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x24
	.long	.LASF103
	.byte	0x3
	.value	0x1b5
	.byte	0x1d
	.long	0x3e6
	.quad	.LFB230
	.quad	.LFE230-.LFB230
	.uleb128 0x1
	.byte	0x9c
	.long	0xc62
	.uleb128 0x25
	.string	"a"
	.byte	0x3
	.value	0x1b5
	.byte	0x36
	.long	0x79
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x25
	.string	"b"
	.byte	0x3
	.value	0x1b5
	.byte	0x3d
	.long	0x79
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x24
	.long	.LASF104
	.byte	0x3
	.value	0x1a8
	.byte	0x1d
	.long	0x3e6
	.quad	.LFB228
	.quad	.LFE228-.LFB228
	.uleb128 0x1
	.byte	0x9c
	.long	0xca2
	.uleb128 0x25
	.string	"a"
	.byte	0x3
	.value	0x1a8
	.byte	0x3e
	.long	0x3e6
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x25
	.string	"b"
	.byte	0x3
	.value	0x1a9
	.byte	0x3e
	.long	0x3e6
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x24
	.long	.LASF105
	.byte	0x3
	.value	0x192
	.byte	0x1d
	.long	0x3e6
	.quad	.LFB226
	.quad	.LFE226-.LFB226
	.uleb128 0x1
	.byte	0x9c
	.long	0xcd4
	.uleb128 0x25
	.string	"a"
	.byte	0x3
	.value	0x192
	.byte	0x43
	.long	0x3e6
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x31
	.long	.LASF106
	.byte	0x3
	.value	0x157
	.byte	0x1d
	.long	0x3e6
	.quad	.LFB221
	.quad	.LFE221-.LFB221
	.uleb128 0x1
	.byte	0x9c
	.long	0xd06
	.uleb128 0x25
	.string	"a"
	.byte	0x3
	.value	0x157
	.byte	0x3f
	.long	0x3e6
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x30
	.long	.LASF108
	.byte	0x2
	.byte	0x77
	.byte	0x14
	.long	0x79
	.quad	.LFB209
	.quad	.LFE209-.LFB209
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x30
	.long	.LASF109
	.byte	0x2
	.byte	0x6b
	.byte	0x14
	.long	0x79
	.quad	.LFB206
	.quad	.LFE206-.LFB206
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x30
	.long	.LASF110
	.byte	0x2
	.byte	0x61
	.byte	0x14
	.long	0x79
	.quad	.LFB204
	.quad	.LFE204-.LFB204
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x30
	.long	.LASF111
	.byte	0x2
	.byte	0x52
	.byte	0x14
	.long	0x79
	.quad	.LFB201
	.quad	.LFE201-.LFB201
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x30
	.long	.LASF112
	.byte	0x2
	.byte	0x3b
	.byte	0x14
	.long	0x79
	.quad	.LFB196
	.quad	.LFE196-.LFB196
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x32
	.long	.LASF113
	.byte	0x2
	.byte	0x30
	.byte	0x20
	.long	0xdba
	.quad	.LFB194
	.quad	.LFE194-.LFB194
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x7
	.byte	0x8
	.long	0xed
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
	.uleb128 0x3
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
	.uleb128 0x4
	.uleb128 0xf
	.byte	0
	.uleb128 0xb
	.uleb128 0xb
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
	.uleb128 0x10
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
	.uleb128 0x11
	.uleb128 0x1
	.byte	0x1
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x12
	.uleb128 0x21
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2f
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x13
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
	.uleb128 0x3f
	.uleb128 0x19
	.uleb128 0x3c
	.uleb128 0x19
	.byte	0
	.byte	0
	.uleb128 0x14
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
	.uleb128 0x15
	.uleb128 0x5
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x16
	.uleb128 0x15
	.byte	0x1
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x17
	.uleb128 0x17
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
	.uleb128 0x18
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
	.byte	0
	.byte	0
	.uleb128 0x19
	.uleb128 0x13
	.byte	0x1
	.uleb128 0xb
	.uleb128 0x5
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
	.uleb128 0x1a
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
	.uleb128 0x5
	.byte	0
	.byte	0
	.uleb128 0x1b
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
	.uleb128 0x5
	.byte	0
	.byte	0
	.uleb128 0x1c
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
	.uleb128 0x1d
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
	.uleb128 0x1e
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
	.uleb128 0x1f
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
	.uleb128 0x20
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
	.uleb128 0x21
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
	.uleb128 0x22
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
	.uleb128 0x23
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
	.uleb128 0x24
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
	.uleb128 0x25
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
	.uleb128 0x28
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x29
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.byte	0
	.byte	0
	.uleb128 0x2a
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
	.uleb128 0x2b
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
	.uleb128 0x2e
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
	.byte	0
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
	.uleb128 0x31
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
	.uleb128 0x32
	.uleb128 0x2e
	.byte	0
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
	.long	0x13c
	.value	0x2
	.long	.Ldebug_info0
	.byte	0x8
	.byte	0
	.value	0
	.value	0
	.quad	.LFB194
	.quad	.LFE194-.LFB194
	.quad	.LFB196
	.quad	.LFE196-.LFB196
	.quad	.LFB201
	.quad	.LFE201-.LFB201
	.quad	.LFB204
	.quad	.LFE204-.LFB204
	.quad	.LFB206
	.quad	.LFE206-.LFB206
	.quad	.LFB209
	.quad	.LFE209-.LFB209
	.quad	.LFB221
	.quad	.LFE221-.LFB221
	.quad	.LFB226
	.quad	.LFE226-.LFB226
	.quad	.LFB228
	.quad	.LFE228-.LFB228
	.quad	.LFB230
	.quad	.LFE230-.LFB230
	.quad	.LFB247
	.quad	.LFE247-.LFB247
	.quad	.LFB249
	.quad	.LFE249-.LFB249
	.quad	.LFB282
	.quad	.LFE282-.LFB282
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
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LFB194
	.quad	.LFE194
	.quad	.LFB196
	.quad	.LFE196
	.quad	.LFB201
	.quad	.LFE201
	.quad	.LFB204
	.quad	.LFE204
	.quad	.LFB206
	.quad	.LFE206
	.quad	.LFB209
	.quad	.LFE209
	.quad	.LFB221
	.quad	.LFE221
	.quad	.LFB226
	.quad	.LFE226
	.quad	.LFB228
	.quad	.LFE228
	.quad	.LFB230
	.quad	.LFE230
	.quad	.LFB247
	.quad	.LFE247
	.quad	.LFB249
	.quad	.LFE249
	.quad	.LFB282
	.quad	.LFE282
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
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF118:
	.string	"aws_lc_0_38_0_OPENSSL_ia32cap_P"
.LASF5:
	.string	"size_t"
.LASF44:
	.string	"md_len"
.LASF45:
	.string	"aes_key_st"
.LASF18:
	.string	"uint64_t"
.LASF6:
	.string	"__uint8_t"
.LASF83:
	.string	"type"
.LASF54:
	.string	"tls_ver"
.LASF30:
	.string	"final"
.LASF19:
	.string	"long long unsigned int"
.LASF21:
	.string	"EVP_CIPHER_CTX"
.LASF111:
	.string	"CRYPTO_is_AESNI_capable"
.LASF32:
	.string	"EVP_CIPHER"
.LASF117:
	.string	"env_md_st"
.LASF87:
	.string	"plen"
.LASF13:
	.string	"long long int"
.LASF4:
	.string	"signed char"
.LASF113:
	.string	"OPENSSL_ia32cap_get"
.LASF28:
	.string	"buf_len"
.LASF120:
	.string	"__PRETTY_FUNCTION__"
.LASF69:
	.string	"aws_lc_0_38_0_EVP_tls_cbc_digest_record"
.LASF102:
	.string	"OPENSSL_memcpy"
.LASF36:
	.string	"iv_len"
.LASF84:
	.string	"u_arg"
.LASF74:
	.string	"aws_lc_0_38_0_SHA256_Update"
.LASF10:
	.string	"long int"
.LASF98:
	.string	"aesni_cbc_hmac_sha256_init_key"
.LASF72:
	.string	"aws_lc_0_38_0_SHA256_Final"
.LASF16:
	.string	"uint16_t"
.LASF49:
	.string	"double"
.LASF110:
	.string	"CRYPTO_is_AMD_XOP_support"
.LASF47:
	.string	"rounds"
.LASF25:
	.string	"key_len"
.LASF8:
	.string	"__uint16_t"
.LASF9:
	.string	"__uint32_t"
.LASF106:
	.string	"constant_time_msb_w"
.LASF59:
	.string	"hmac_key"
.LASF104:
	.string	"constant_time_eq_w"
.LASF67:
	.string	"aws_lc_0_38_0_EVP_tls_cbc_copy_mac"
.LASF1:
	.string	"unsigned int"
.LASF55:
	.string	"tls_aad"
.LASF96:
	.string	"record_mac"
.LASF50:
	.string	"__int128"
.LASF0:
	.string	"long unsigned int"
.LASF107:
	.string	"hwaes_capable"
.LASF77:
	.string	"aws_lc_0_38_0_SHA256_Init"
.LASF26:
	.string	"encrypt"
.LASF80:
	.string	"aws_lc_0_38_0_EVP_CIPHER_CTX_key_length"
.LASF42:
	.string	"sha256_state_st"
.LASF3:
	.string	"short unsigned int"
.LASF89:
	.string	"blocks"
.LASF71:
	.string	"aws_lc_0_38_0_aes_hw_cbc_encrypt"
.LASF7:
	.string	"short int"
.LASF95:
	.string	"record_mac_tmp"
.LASF46:
	.string	"rd_key"
.LASF35:
	.string	"block_size"
.LASF11:
	.string	"__uint64_t"
.LASF20:
	.string	"EVP_MD"
.LASF57:
	.string	"tail"
.LASF34:
	.string	"evp_cipher_st"
.LASF99:
	.string	"inkey"
.LASF43:
	.string	"data"
.LASF73:
	.string	"aws_lc_0_38_0_aesni_cbc_sha256_enc"
.LASF112:
	.string	"CRYPTO_is_intel_cpu"
.LASF92:
	.string	"padding_ok"
.LASF39:
	.string	"cleanup"
.LASF62:
	.string	"aesni_256_cbc_hmac_sha256_cipher"
.LASF103:
	.string	"constant_time_eq_int"
.LASF38:
	.string	"init"
.LASF70:
	.string	"aws_lc_0_38_0_EVP_tls_cbc_remove_padding"
.LASF76:
	.string	"aws_lc_0_38_0_ERR_put_error"
.LASF119:
	.string	"aws_lc_0_38_0_EVP_sha256"
.LASF68:
	.string	"__assert_fail"
.LASF81:
	.string	"aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha256"
.LASF115:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aes_cbc_hmac_sha256.c"
.LASF51:
	.string	"__int128 unsigned"
.LASF53:
	.string	"_Bool"
.LASF2:
	.string	"unsigned char"
.LASF116:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF58:
	.string	"payload_length"
.LASF23:
	.string	"app_data"
.LASF94:
	.string	"mac_len"
.LASF114:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF93:
	.string	"data_len"
.LASF37:
	.string	"ctx_size"
.LASF52:
	.string	"crypto_word_t"
.LASF41:
	.string	"SHA256_CTX"
.LASF79:
	.string	"aws_lc_0_38_0_aes_hw_set_encrypt_key"
.LASF56:
	.string	"head"
.LASF91:
	.string	"data_plus_mac_len"
.LASF14:
	.string	"long double"
.LASF12:
	.string	"char"
.LASF101:
	.string	"OPENSSL_memset"
.LASF24:
	.string	"cipher_data"
.LASF60:
	.string	"EVP_AES_HMAC_SHA256"
.LASF108:
	.string	"CRYPTO_is_SHAEXT_capable"
.LASF100:
	.string	"key_bits"
.LASF85:
	.string	"aesni_cbc_hmac_sha256_ctrl"
.LASF31:
	.string	"poisoned"
.LASF40:
	.string	"ctrl"
.LASF64:
	.string	"memcpy"
.LASF75:
	.string	"aws_lc_0_38_0_EVP_CIPHER_CTX_encrypting"
.LASF90:
	.string	"sha_off"
.LASF17:
	.string	"uint32_t"
.LASF109:
	.string	"CRYPTO_is_AVX2_capable"
.LASF63:
	.string	"memset"
.LASF97:
	.string	"good"
.LASF66:
	.string	"aws_lc_0_38_0_OPENSSL_cleanse"
.LASF61:
	.string	"aesni_128_cbc_hmac_sha256_cipher"
.LASF15:
	.string	"uint8_t"
.LASF27:
	.string	"flags"
.LASF22:
	.string	"cipher"
.LASF88:
	.string	"aes_off"
.LASF82:
	.string	"aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha256"
.LASF48:
	.string	"AES_KEY"
.LASF78:
	.string	"aws_lc_0_38_0_aes_hw_set_decrypt_key"
.LASF29:
	.string	"final_used"
.LASF86:
	.string	"aesni_cbc_hmac_sha256_cipher"
.LASF33:
	.string	"evp_cipher_ctx_st"
.LASF105:
	.string	"constant_time_is_zero_w"
.LASF65:
	.string	"aws_lc_0_38_0_CRYPTO_memcmp"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
