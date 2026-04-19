	.file	"e_aes_cbc_hmac_sha1.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aes_cbc_hmac_sha1.c"
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
	.section	.text.CRYPTO_is_AVX_capable,"ax",@progbits
	.type	CRYPTO_is_AVX_capable, @function
CRYPTO_is_AVX_capable:
.LFB202:
	.loc 2 89 48
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 2 90 11
	call	OPENSSL_ia32cap_get
	.loc 2 90 32 discriminator 1
	addq	$4, %rax
	movl	(%rax), %eax
	.loc 2 90 36 discriminator 1
	andl	$268435456, %eax
	.loc 2 90 49 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 2 91 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE202:
	.size	CRYPTO_is_AVX_capable, .-CRYPTO_is_AVX_capable
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
	.section	.text.aesni_cbc_hmac_sha1_init_key,"ax",@progbits
	.type	aesni_cbc_hmac_sha1_init_key, @function
aesni_cbc_hmac_sha1_init_key:
.LFB292:
	.loc 1 63 69
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$64, %rsp
	movq	%rdi, 24(%rsp)
	movq	%rsi, 16(%rsp)
	movq	%rdx, 8(%rsp)
	movl	%ecx, 4(%rsp)
	.loc 1 64 22
	movq	24(%rsp), %rax
	movq	16(%rax), %rax
	movq	%rax, 48(%rsp)
	.loc 1 67 18
	movq	24(%rsp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_CIPHER_CTX_key_length@PLT
	.loc 1 67 49 discriminator 1
	sall	$3, %eax
	.loc 1 67 7 discriminator 1
	movl	%eax, 44(%rsp)
	.loc 1 68 6
	cmpl	$0, 4(%rsp)
	je	.L30
	.loc 1 69 11
	movq	48(%rsp), %rdx
	movl	44(%rsp), %ecx
	movq	16(%rsp), %rax
	movl	%ecx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes_hw_set_encrypt_key@PLT
	movl	%eax, 60(%rsp)
	jmp	.L31
.L30:
	.loc 1 71 11
	movq	48(%rsp), %rdx
	movl	44(%rsp), %ecx
	movq	16(%rsp), %rax
	movl	%ecx, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes_hw_set_decrypt_key@PLT
	movl	%eax, 60(%rsp)
.L31:
	.loc 1 73 6
	cmpl	$0, 60(%rsp)
	jns	.L32
	.loc 1 74 12
	movl	$0, %eax
	jmp	.L33
.L32:
	.loc 1 77 3
	movq	48(%rsp), %rax
	addq	$244, %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA1_Init@PLT
	.loc 1 78 13
	movq	48(%rsp), %rax
	movq	48(%rsp), %rdx
	vmovdqu	244(%rdx), %ymm0
	vmovdqu	%ymm0, 340(%rax)
	vmovdqu	276(%rdx), %ymm0
	vmovdqu	%ymm0, 372(%rax)
	vmovdqu	308(%rdx), %ymm0
	vmovdqu	%ymm0, 404(%rax)
	.loc 1 79 11
	movq	48(%rsp), %rax
	movq	48(%rsp), %rdx
	vmovdqu	244(%rdx), %ymm0
	vmovdqu	%ymm0, 436(%rax)
	vmovdqu	276(%rdx), %ymm0
	vmovdqu	%ymm0, 468(%rax)
	vmovdqu	308(%rdx), %ymm0
	vmovdqu	%ymm0, 500(%rax)
	.loc 1 81 23
	movq	48(%rsp), %rax
	movq	$-1, 536(%rax)
	.loc 1 83 10
	movl	$1, %eax
.L33:
	.loc 1 84 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE292:
	.size	aesni_cbc_hmac_sha1_init_key, .-aesni_cbc_hmac_sha1_init_key
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aes_cbc_hmac_sha1.c"
.LC1:
	.string	"mac_len == SHA_DIGEST_LENGTH"
	.section	.text.aesni_cbc_hmac_sha1_cipher,"ax",@progbits
	.type	aesni_cbc_hmac_sha1_cipher, @function
aesni_cbc_hmac_sha1_cipher:
.LFB293:
	.loc 1 106 70
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
	.loc 1 107 22
	movq	-280(%rbp), %rax
	movq	16(%rax), %rax
	movq	%rax, -72(%rbp)
	.loc 1 108 10
	movq	-72(%rbp), %rax
	movq	536(%rax), %rax
	movq	%rax, -40(%rbp)
	.loc 1 108 38
	movq	$0, -48(%rbp)
	.loc 1 110 23
	movq	-72(%rbp), %rax
	movq	$-1, 536(%rax)
	.loc 1 112 11
	movq	-304(%rbp), %rax
	andl	$15, %eax
	.loc 1 112 6
	testq	%rax, %rax
	je	.L35
	.loc 1 113 5
	movl	$113, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$106, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 114 12
	movl	$0, %eax
	jmp	.L36
.L35:
	.loc 1 117 7
	movq	-280(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_CIPHER_CTX_encrypting@PLT
	.loc 1 117 6 discriminator 1
	testl	%eax, %eax
	je	.L37
.LBB2:
	.loc 1 127 8
	cmpq	$-1, -40(%rbp)
	jne	.L38
	.loc 1 130 7
	movl	$130, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$140, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 131 14
	movl	$0, %eax
	jmp	.L36
.L38:
	.loc 1 134 36
	movq	-40(%rbp), %rax
	addq	$36, %rax
	.loc 1 134 54
	andq	$-16, %rax
	.loc 1 133 8
	cmpq	%rax, -304(%rbp)
	je	.L39
	.loc 1 136 7
	movl	$136, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$119, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 137 14
	movl	$0, %eax
	jmp	.L36
.L39:
	.loc 1 138 24
	movq	-72(%rbp), %rax
	movzwl	544(%rax), %eax
	.loc 1 138 15
	cmpw	$769, %ax
	jbe	.L40
	.loc 1 139 14
	movq	$16, -48(%rbp)
.L40:
	.loc 1 142 12
	movq	$0, -56(%rbp)
	.loc 1 143 42
	movq	-72(%rbp), %rax
	movl	528(%rax), %eax
	.loc 1 143 33
	movl	$64, %edx
	subl	%eax, %edx
	.loc 1 143 12
	movl	%edx, %eax
	movq	%rax, -64(%rbp)
	.loc 1 157 10
	call	CRYPTO_is_SHAEXT_capable
	.loc 1 157 8 discriminator 1
	testl	%eax, %eax
	jne	.L41
	.loc 1 158 10
	call	CRYPTO_is_AVX_capable
	.loc 1 157 37 discriminator 1
	testl	%eax, %eax
	je	.L42
	.loc 1 159 10
	call	CRYPTO_is_AMD_XOP_support
	movl	%eax, %ebx
	.loc 1 159 40 discriminator 1
	call	CRYPTO_is_intel_cpu
	.loc 1 159 38 discriminator 2
	orl	%ebx, %eax
	.loc 1 158 34
	testl	%eax, %eax
	je	.L42
.L41:
	.loc 1 160 25
	movq	-64(%rbp), %rdx
	movq	-48(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 159 65
	cmpq	-40(%rbp), %rax
	jnb	.L42
	.loc 1 161 36
	movq	-64(%rbp), %rdx
	movq	-48(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 161 25
	movq	-40(%rbp), %rdx
	subq	%rax, %rdx
	.loc 1 161 17
	movq	%rdx, %rax
	shrq	$6, %rax
	movq	%rax, -104(%rbp)
	.loc 1 160 35
	cmpq	$0, -104(%rbp)
	je	.L42
	.loc 1 164 32
	movq	-296(%rbp), %rdx
	movq	-48(%rbp), %rax
	leaq	(%rdx,%rax), %rsi
	.loc 1 164 7
	movq	-72(%rbp), %rax
	leaq	436(%rax), %rcx
	movq	-64(%rbp), %rax
	movq	%rax, %rdx
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA1_Update@PLT
	.loc 1 168 38
	movq	-48(%rbp), %rdx
	movq	-64(%rbp), %rax
	addq	%rax, %rdx
	movq	-296(%rbp), %rax
	leaq	(%rdx,%rax), %rdi
	.loc 1 166 7
	movq	-72(%rbp), %rax
	leaq	436(%rax), %r9
	.loc 1 167 29
	movq	-280(%rbp), %rax
	leaq	52(%rax), %r8
	.loc 1 166 43
	movq	-72(%rbp), %rcx
	.loc 1 166 7
	movq	-104(%rbp), %rdx
	movq	-288(%rbp), %rsi
	movq	-296(%rbp), %rax
	subq	$8, %rsp
	pushq	%rdi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesni_cbc_sha1_enc@PLT
	addq	$16, %rsp
	.loc 1 171 14
	salq	$6, -104(%rbp)
	.loc 1 172 15
	movq	-104(%rbp), %rax
	addq	%rax, -56(%rbp)
	.loc 1 173 15
	movq	-104(%rbp), %rax
	addq	%rax, -64(%rbp)
	.loc 1 174 14
	movq	-72(%rbp), %rax
	movl	460(%rax), %eax
	.loc 1 174 28
	movq	-104(%rbp), %rdx
	shrq	$29, %rdx
	.loc 1 174 18
	addl	%eax, %edx
	movq	-72(%rbp), %rax
	movl	%edx, 460(%rax)
	.loc 1 175 28
	salq	$3, -104(%rbp)
	movq	-104(%rbp), %rdx
	.loc 1 175 14
	movq	-72(%rbp), %rax
	movl	456(%rax), %eax
	.loc 1 175 18
	addl	%eax, %edx
	movq	-72(%rbp), %rax
	movl	%edx, 456(%rax)
	.loc 1 176 18
	movq	-72(%rbp), %rax
	movl	456(%rax), %eax
	.loc 1 176 24
	movq	-104(%rbp), %rdx
	.loc 1 176 10
	cmpl	%edx, %eax
	jnb	.L44
	.loc 1 177 16
	movq	-72(%rbp), %rax
	movl	460(%rax), %eax
	.loc 1 177 19
	leal	1(%rax), %edx
	movq	-72(%rbp), %rax
	movl	%edx, 460(%rax)
	.loc 1 176 10
	jmp	.L44
.L42:
	.loc 1 180 15
	movq	$0, -64(%rbp)
.L44:
	.loc 1 182 13
	movq	-48(%rbp), %rax
	addq	%rax, -64(%rbp)
	.loc 1 183 5
	movq	-40(%rbp), %rax
	subq	-64(%rbp), %rax
	.loc 1 183 30
	movq	-296(%rbp), %rcx
	movq	-64(%rbp), %rdx
	leaq	(%rcx,%rdx), %rsi
	.loc 1 183 5
	movq	-72(%rbp), %rdx
	leaq	436(%rdx), %rcx
	movq	%rax, %rdx
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA1_Update@PLT
	.loc 1 185 8
	movq	-296(%rbp), %rax
	cmpq	-288(%rbp), %rax
	je	.L45
	.loc 1 186 7
	movq	-40(%rbp), %rax
	subq	-56(%rbp), %rax
	.loc 1 186 40
	movq	-296(%rbp), %rcx
	movq	-56(%rbp), %rdx
	leaq	(%rcx,%rdx), %rsi
	.loc 1 186 26
	movq	-288(%rbp), %rcx
	movq	-56(%rbp), %rdx
	addq	%rdx, %rcx
	.loc 1 186 7
	movq	%rax, %rdx
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
.L45:
	.loc 1 190 5
	movq	-72(%rbp), %rax
	leaq	436(%rax), %rdx
	movq	-288(%rbp), %rcx
	movq	-40(%rbp), %rax
	addq	%rcx, %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA1_Final@PLT
	.loc 1 191 13
	movq	-72(%rbp), %rax
	movq	-72(%rbp), %rdx
	vmovdqu	340(%rdx), %ymm0
	vmovdqu	%ymm0, 436(%rax)
	vmovdqu	372(%rdx), %ymm0
	vmovdqu	%ymm0, 468(%rax)
	vmovdqu	404(%rdx), %ymm0
	vmovdqu	%ymm0, 500(%rax)
	.loc 1 192 31
	movq	-288(%rbp), %rdx
	movq	-40(%rbp), %rax
	leaq	(%rdx,%rax), %rcx
	.loc 1 192 5
	movq	-72(%rbp), %rax
	addq	$436, %rax
	movl	$20, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA1_Update@PLT
	.loc 1 193 5
	movq	-72(%rbp), %rax
	leaq	436(%rax), %rdx
	movq	-288(%rbp), %rcx
	movq	-40(%rbp), %rax
	addq	%rcx, %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA1_Final@PLT
	.loc 1 196 10
	addq	$20, -40(%rbp)
.LBB3:
	.loc 1 197 31
	movq	-304(%rbp), %rax
	movl	%eax, %ecx
	movq	-40(%rbp), %rax
	movl	%eax, %edx
	movl	%ecx, %eax
	subl	%edx, %eax
	.loc 1 197 23
	subl	$1, %eax
	movl	%eax, -108(%rbp)
	.loc 1 197 5
	jmp	.L46
.L47:
	.loc 1 198 10
	movq	-288(%rbp), %rdx
	movq	-40(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 198 17
	movl	-108(%rbp), %edx
	movb	%dl, (%rax)
	.loc 1 197 59 discriminator 3
	addq	$1, -40(%rbp)
.L46:
	.loc 1 197 48 discriminator 1
	movq	-40(%rbp), %rax
	cmpq	-304(%rbp), %rax
	jb	.L47
.LBE3:
	.loc 1 202 27
	movq	-280(%rbp), %rax
	leaq	52(%rax), %r8
	.loc 1 201 69
	movq	-72(%rbp), %rdx
	.loc 1 201 5
	movq	-304(%rbp), %rax
	subq	-56(%rbp), %rax
	movq	-288(%rbp), %rsi
	movq	-56(%rbp), %rcx
	addq	%rcx, %rsi
	.loc 1 201 28
	movq	-288(%rbp), %rdi
	movq	-56(%rbp), %rcx
	addq	%rcx, %rdi
	.loc 1 201 5
	movl	$1, %r9d
	movq	%rdx, %rcx
	movq	%rax, %rdx
	call	aws_lc_0_38_0_aes_hw_cbc_encrypt@PLT
	.loc 1 203 12
	movl	$1, %eax
	jmp	.L36
.L37:
.LBE2:
.LBB4:
	.loc 1 205 8
	cmpq	$13, -40(%rbp)
	je	.L48
	.loc 1 208 7
	movl	$208, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$140, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 209 14
	movl	$0, %eax
	jmp	.L36
.L48:
	.loc 1 212 32
	movq	-40(%rbp), %rax
	leaq	-4(%rax), %rdx
	.loc 1 212 26
	movq	-72(%rbp), %rax
	movzbl	544(%rax,%rdx), %eax
	movzbl	%al, %eax
	.loc 1 212 37
	sall	$8, %eax
	movl	%eax, %ecx
	.loc 1 212 66
	movq	-40(%rbp), %rax
	leaq	-3(%rax), %rdx
	.loc 1 212 60
	movq	-72(%rbp), %rax
	movzbl	544(%rax,%rdx), %eax
	movzbl	%al, %eax
	.loc 1 212 42
	orl	%ecx, %eax
	.loc 1 212 8
	cmpl	$769, %eax
	jle	.L50
	.loc 1 214 10
	cmpq	$36, -304(%rbp)
	ja	.L51
	.loc 1 215 9
	movl	$215, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$119, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 216 16
	movl	$0, %eax
	jmp	.L36
.L51:
	.loc 1 220 25
	movq	-280(%rbp), %rax
	leaq	52(%rax), %rcx
	.loc 1 220 7
	movq	-296(%rbp), %rax
	movl	$16, %edx
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
	.loc 1 221 10
	addq	$16, -296(%rbp)
	.loc 1 222 11
	addq	$16, -288(%rbp)
	.loc 1 223 11
	subq	$16, -304(%rbp)
	jmp	.L52
.L50:
	.loc 1 224 15
	cmpq	$20, -304(%rbp)
	ja	.L52
	.loc 1 225 7
	movl	$225, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$119, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 226 14
	movl	$0, %eax
	jmp	.L36
.L52:
	.loc 1 229 51
	movq	-280(%rbp), %rax
	leaq	52(%rax), %rdi
	.loc 1 229 38
	movq	-72(%rbp), %rcx
	.loc 1 229 5
	movq	-304(%rbp), %rdx
	movq	-288(%rbp), %rsi
	movq	-296(%rbp), %rax
	movl	$0, %r9d
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes_hw_cbc_encrypt@PLT
	.loc 1 237 10
	movq	-304(%rbp), %rcx
	movq	-288(%rbp), %rdx
	leaq	-120(%rbp), %rsi
	leaq	-128(%rbp), %rax
	movl	$20, %r9d
	movl	$16, %r8d
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_tls_cbc_remove_padding@PLT
	.loc 1 237 8 discriminator 1
	testl	%eax, %eax
	jne	.L53
	.loc 1 240 7
	movl	$240, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 241 14
	movl	$0, %eax
	jmp	.L36
.L53:
	.loc 1 244 41
	movq	-120(%rbp), %rax
	.loc 1 244 12
	subq	$20, %rax
	movq	%rax, -80(%rbp)
	.loc 1 246 47
	movq	-80(%rbp), %rax
	shrq	$8, %rax
	.loc 1 246 28
	movl	%eax, %edx
	.loc 1 246 26
	movq	-72(%rbp), %rax
	movb	%dl, 555(%rax)
	.loc 1 247 28
	movq	-80(%rbp), %rax
	movl	%eax, %edx
	.loc 1 247 26
	movq	-72(%rbp), %rax
	movb	%dl, 556(%rax)
	.loc 1 255 59
	movq	-72(%rbp), %rax
	leaq	558(%rax), %r12
	.loc 1 254 71
	movq	-72(%rbp), %rax
	leaq	544(%rax), %rbx
	.loc 1 254 10
	call	aws_lc_0_38_0_EVP_sha1@PLT
	movq	%rax, %rdi
	.loc 1 254 10 is_stmt 0 discriminator 1
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
	.loc 1 254 8 is_stmt 1 discriminator 2
	testl	%eax, %eax
	jne	.L54
	.loc 1 256 7
	movl	$256, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 257 14
	movl	$0, %eax
	jmp	.L36
.L54:
	.loc 1 259 5
	movq	-136(%rbp), %rax
	cmpq	$20, %rax
	je	.L55
	.loc 1 259 5 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$259, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC1(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L55:
	.loc 1 261 16 is_stmt 1
	leaq	-208(%rbp), %rax
	movq	%rax, -88(%rbp)
	.loc 1 262 5
	movq	-120(%rbp), %rcx
	movq	-136(%rbp), %rsi
	movq	-304(%rbp), %rdi
	movq	-288(%rbp), %rdx
	movq	-88(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_tls_cbc_copy_mac@PLT
	.loc 1 269 9
	movq	-136(%rbp), %rdx
	leaq	-272(%rbp), %rcx
	movq	-88(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_memcmp@PLT
	.loc 1 269 9 is_stmt 0 discriminator 1
	movl	$0, %esi
	movl	%eax, %edi
	call	constant_time_eq_int
	movq	%rax, -96(%rbp)
	.loc 1 270 10 is_stmt 1
	movq	-128(%rbp), %rax
	andq	%rax, -96(%rbp)
	.loc 1 272 8
	cmpq	$0, -96(%rbp)
	jne	.L56
	.loc 1 273 7
	movl	$273, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 274 14
	movl	$0, %eax
	jmp	.L36
.L56:
	.loc 1 281 12
	movl	$1, %eax
.L36:
.LBE4:
	.loc 1 283 1
	leaq	-24(%rbp), %rsp
	popq	%rbx
	popq	%r10
	popq	%r12
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE293:
	.size	aesni_cbc_hmac_sha1_cipher, .-aesni_cbc_hmac_sha1_cipher
	.section	.text.aesni_cbc_hmac_sha1_ctrl,"ax",@progbits
	.type	aesni_cbc_hmac_sha1_ctrl, @function
aesni_cbc_hmac_sha1_ctrl:
.LFB294:
	.loc 1 286 48
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$144, %rsp
	movq	%rdi, 24(%rsp)
	movl	%esi, 20(%rsp)
	movl	%edx, 16(%rsp)
	movq	%rcx, 8(%rsp)
	.loc 1 287 22
	movq	24(%rsp), %rax
	movq	16(%rax), %rax
	movq	%rax, 112(%rsp)
	.loc 1 289 3
	cmpl	$22, 20(%rsp)
	je	.L58
	cmpl	$23, 20(%rsp)
	jne	.L59
.LBB5:
	.loc 1 291 10
	cmpl	$0, 16(%rsp)
	jns	.L60
	.loc 1 292 16
	movl	$0, %eax
	jmp	.L68
.L60:
	.loc 1 296 7
	leaq	32(%rsp), %rax
	movl	$64, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 297 14
	movl	16(%rsp), %eax
	cltq
	movq	%rax, 104(%rsp)
	.loc 1 298 10
	cmpq	$64, 104(%rsp)
	jbe	.L62
	.loc 1 299 9
	movq	112(%rsp), %rax
	addq	$244, %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA1_Init@PLT
	.loc 1 300 9
	movl	16(%rsp), %eax
	movslq	%eax, %rdx
	movq	112(%rsp), %rax
	leaq	244(%rax), %rcx
	movq	8(%rsp), %rax
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA1_Update@PLT
	.loc 1 301 9
	movq	112(%rsp), %rax
	leaq	244(%rax), %rdx
	leaq	32(%rsp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA1_Final@PLT
	jmp	.L63
.L62:
	.loc 1 303 9
	movl	16(%rsp), %eax
	movslq	%eax, %rdx
	movq	8(%rsp), %rcx
	leaq	32(%rsp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
.L63:
	.loc 1 305 22
	movq	112(%rsp), %rax
	leaq	558(%rax), %rcx
	.loc 1 305 7
	leaq	32(%rsp), %rax
	movl	$64, %edx
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
.LBB6:
	.loc 1 307 19
	movq	$0, 136(%rsp)
	.loc 1 307 7
	jmp	.L64
.L65:
	.loc 1 308 17
	leaq	32(%rsp), %rdx
	movq	136(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	.loc 1 308 21
	xorl	$54, %eax
	movl	%eax, %edx
	leaq	32(%rsp), %rcx
	movq	136(%rsp), %rax
	addq	%rcx, %rax
	movb	%dl, (%rax)
	.loc 1 307 49 discriminator 3
	addq	$1, 136(%rsp)
.L64:
	.loc 1 307 28 discriminator 1
	cmpq	$63, 136(%rsp)
	jbe	.L65
.LBE6:
	.loc 1 310 7
	movq	112(%rsp), %rax
	addq	$244, %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA1_Init@PLT
	.loc 1 311 7
	movq	112(%rsp), %rax
	leaq	244(%rax), %rcx
	leaq	32(%rsp), %rax
	movl	$64, %edx
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA1_Update@PLT
.LBB7:
	.loc 1 313 19
	movq	$0, 128(%rsp)
	.loc 1 313 7
	jmp	.L66
.L67:
	.loc 1 314 17
	leaq	32(%rsp), %rdx
	movq	128(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	.loc 1 314 21
	xorl	$106, %eax
	movl	%eax, %edx
	leaq	32(%rsp), %rcx
	movq	128(%rsp), %rax
	addq	%rcx, %rax
	movb	%dl, (%rax)
	.loc 1 313 49 discriminator 3
	addq	$1, 128(%rsp)
.L66:
	.loc 1 313 28 discriminator 1
	cmpq	$63, 128(%rsp)
	jbe	.L67
.LBE7:
	.loc 1 316 7
	movq	112(%rsp), %rax
	addq	$340, %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA1_Init@PLT
	.loc 1 317 7
	movq	112(%rsp), %rax
	leaq	340(%rax), %rcx
	leaq	32(%rsp), %rax
	movl	$64, %edx
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA1_Update@PLT
	.loc 1 319 7
	leaq	32(%rsp), %rax
	movl	$64, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_OPENSSL_cleanse@PLT
	.loc 1 321 14
	movl	$1, %eax
	jmp	.L68
.L58:
.LBE5:
.LBB8:
	.loc 1 330 16
	movq	8(%rsp), %rax
	movq	%rax, 96(%rsp)
	.loc 1 331 10
	cmpl	$13, 16(%rsp)
	je	.L69
	.loc 1 332 9
	movl	$332, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$109, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 333 16
	movl	$0, %eax
	jmp	.L68
.L69:
	.loc 1 336 11
	movq	24(%rsp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_EVP_CIPHER_CTX_encrypting@PLT
	.loc 1 336 10 discriminator 1
	testl	%eax, %eax
	je	.L70
.LBB9:
	.loc 1 337 25
	movl	16(%rsp), %eax
	cltq
	leaq	-2(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	.loc 1 337 40
	movzbl	%al, %eax
	sall	$8, %eax
	movl	%eax, %ecx
	.loc 1 337 43
	movl	16(%rsp), %eax
	cltq
	leaq	-1(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	movzbl	%al, %eax
	.loc 1 337 40
	orl	%ecx, %eax
	.loc 1 337 18
	movw	%ax, 126(%rsp)
	.loc 1 338 29
	movzwl	126(%rsp), %edx
	movq	112(%rsp), %rax
	movq	%rdx, 536(%rax)
	.loc 1 339 34
	movl	16(%rsp), %eax
	cltq
	leaq	-4(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	.loc 1 339 49
	movzbl	%al, %eax
	sall	$8, %eax
	movl	%eax, %ecx
	.loc 1 339 52
	movl	16(%rsp), %eax
	cltq
	leaq	-3(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	movzbl	%al, %eax
	.loc 1 339 49
	orl	%ecx, %eax
	movl	%eax, %edx
	.loc 1 339 31
	movq	112(%rsp), %rax
	movw	%dx, 544(%rax)
	.loc 1 339 22
	movq	112(%rsp), %rax
	movzwl	544(%rax), %eax
	.loc 1 339 12
	cmpw	$769, %ax
	jbe	.L71
	.loc 1 341 14
	cmpw	$15, 126(%rsp)
	ja	.L72
	.loc 1 342 13
	movl	$342, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$109, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 343 20
	movl	$0, %eax
	jmp	.L68
.L72:
	.loc 1 345 15
	subw	$16, 126(%rsp)
	.loc 1 346 22
	movzwl	126(%rsp), %eax
	shrw	$8, %ax
	movl	%eax, %ecx
	.loc 1 346 12
	movl	16(%rsp), %eax
	cltq
	leaq	-2(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	.loc 1 346 22
	movl	%ecx, %edx
	movb	%dl, (%rax)
	.loc 1 347 12
	movl	16(%rsp), %eax
	cltq
	leaq	-1(%rax), %rdx
	movq	96(%rsp), %rax
	addq	%rdx, %rax
	.loc 1 347 22
	movzwl	126(%rsp), %edx
	movb	%dl, (%rax)
.L71:
	.loc 1 349 17
	movq	112(%rsp), %rax
	movq	112(%rsp), %rdx
	vmovdqu	244(%rdx), %ymm0
	vmovdqu	%ymm0, 436(%rax)
	vmovdqu	276(%rdx), %ymm0
	vmovdqu	%ymm0, 468(%rax)
	vmovdqu	308(%rdx), %ymm0
	vmovdqu	%ymm0, 500(%rax)
	.loc 1 350 9
	movl	16(%rsp), %eax
	movslq	%eax, %rdx
	movq	112(%rsp), %rax
	leaq	436(%rax), %rcx
	movq	96(%rsp), %rax
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	aws_lc_0_38_0_SHA1_Update@PLT
	.loc 1 352 48
	movzwl	126(%rsp), %eax
	addl	$36, %eax
	.loc 1 352 66
	andl	$-16, %eax
	.loc 1 352 16
	movzwl	126(%rsp), %edx
	subl	%edx, %eax
	jmp	.L68
.L70:
.LBE9:
	.loc 1 356 9
	movl	16(%rsp), %eax
	movslq	%eax, %rdx
	.loc 1 356 32
	movq	112(%rsp), %rax
	leaq	544(%rax), %rcx
	.loc 1 356 9
	movq	8(%rsp), %rax
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
	.loc 1 357 29
	movl	16(%rsp), %eax
	movslq	%eax, %rdx
	movq	112(%rsp), %rax
	movq	%rdx, 536(%rax)
	.loc 1 359 16
	movl	$20, %eax
	jmp	.L68
.L59:
.LBE8:
	.loc 1 363 7
	movl	$363, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$104, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 364 14
	movl	$0, %eax
.L68:
	.loc 1 366 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE294:
	.size	aesni_cbc_hmac_sha1_ctrl, .-aesni_cbc_hmac_sha1_ctrl
	.section	.data.rel.ro.local.aesni_128_cbc_hmac_sha1_cipher,"aw"
	.align 32
	.type	aesni_128_cbc_hmac_sha1_cipher, @object
	.size	aesni_128_cbc_hmac_sha1_cipher, 56
aesni_128_cbc_hmac_sha1_cipher:
	.long	916
	.long	16
	.long	16
	.long	16
	.long	624
	.long	2050
	.quad	aesni_cbc_hmac_sha1_init_key
	.quad	aesni_cbc_hmac_sha1_cipher
	.quad	0
	.quad	aesni_cbc_hmac_sha1_ctrl
	.section	.data.rel.ro.local.aesni_256_cbc_hmac_sha1_cipher,"aw"
	.align 32
	.type	aesni_256_cbc_hmac_sha1_cipher, @object
	.size	aesni_256_cbc_hmac_sha1_cipher, 56
aesni_256_cbc_hmac_sha1_cipher:
	.long	918
	.long	16
	.long	32
	.long	16
	.long	624
	.long	2050
	.quad	aesni_cbc_hmac_sha1_init_key
	.quad	aesni_cbc_hmac_sha1_cipher
	.quad	0
	.quad	aesni_cbc_hmac_sha1_ctrl
	.section	.text.aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha1,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha1
	.type	aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha1, @function
aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha1:
.LFB295:
	.loc 1 392 51
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 1 393 11
	call	hwaes_capable
	.loc 1 393 61 discriminator 1
	testl	%eax, %eax
	je	.L74
	leaq	aesni_128_cbc_hmac_sha1_cipher(%rip), %rax
	.loc 1 393 61 is_stmt 0
	jmp	.L76
.L74:
	.loc 1 393 61 discriminator 2
	movl	$0, %eax
.L76:
	.loc 1 394 1 is_stmt 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE295:
	.size	aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha1, .-aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha1
	.section	.text.aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha1,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha1
	.type	aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha1, @function
aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha1:
.LFB296:
	.loc 1 396 51
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 1 397 11
	call	hwaes_capable
	.loc 1 397 61 discriminator 1
	testl	%eax, %eax
	je	.L78
	leaq	aesni_256_cbc_hmac_sha1_cipher(%rip), %rax
	.loc 1 397 61 is_stmt 0
	jmp	.L80
.L78:
	.loc 1 397 61 discriminator 2
	movl	$0, %eax
.L80:
	.loc 1 398 1 is_stmt 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE296:
	.size	aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha1, .-aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha1
	.section	.rodata.__PRETTY_FUNCTION__.0,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.0, @object
	.size	__PRETTY_FUNCTION__.0, 27
__PRETTY_FUNCTION__.0:
	.string	"aesni_cbc_hmac_sha1_cipher"
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
	.long	0xdea
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF113
	.byte	0xc
	.long	.LASF114
	.long	.LASF115
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
	.long	.LASF116
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
	.long	0x34a
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
	.long	0x350
	.byte	0x24
	.uleb128 0xd
	.string	"iv"
	.byte	0x9
	.value	0x28a
	.byte	0xb
	.long	0x350
	.byte	0x34
	.uleb128 0xd
	.string	"buf"
	.byte	0x9
	.value	0x28e
	.byte	0xb
	.long	0x360
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
	.long	0x360
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
	.long	0x41d
	.byte	0x18
	.uleb128 0x10
	.long	.LASF22
	.byte	0xa
	.byte	0xb5
	.byte	0x9
	.long	0x441
	.byte	0x20
	.uleb128 0x10
	.long	.LASF39
	.byte	0xa
	.byte	0xbb
	.byte	0xa
	.long	0x452
	.byte	0x28
	.uleb128 0x10
	.long	.LASF40
	.byte	0xa
	.byte	0xbd
	.byte	0x9
	.long	0x476
	.byte	0x30
	.byte	0
	.uleb128 0x9
	.long	.LASF41
	.byte	0x8
	.value	0x1d7
	.byte	0x1d
	.long	0x2b6
	.uleb128 0xe
	.long	.LASF42
	.byte	0x60
	.byte	0xb
	.byte	0x63
	.byte	0x8
	.long	0x301
	.uleb128 0xf
	.string	"h"
	.byte	0xb
	.byte	0x64
	.byte	0xc
	.long	0x370
	.byte	0
	.uleb128 0xf
	.string	"Nl"
	.byte	0xb
	.byte	0x65
	.byte	0xc
	.long	0xe1
	.byte	0x14
	.uleb128 0xf
	.string	"Nh"
	.byte	0xb
	.byte	0x65
	.byte	0x10
	.long	0xe1
	.byte	0x18
	.uleb128 0x10
	.long	.LASF43
	.byte	0xb
	.byte	0x66
	.byte	0xb
	.long	0x380
	.byte	0x1c
	.uleb128 0xf
	.string	"num"
	.byte	0xb
	.byte	0x67
	.byte	0xc
	.long	0x3c
	.byte	0x5c
	.byte	0
	.uleb128 0xe
	.long	.LASF44
	.byte	0xf4
	.byte	0xc
	.byte	0x48
	.byte	0x8
	.long	0x329
	.uleb128 0x10
	.long	.LASF45
	.byte	0xc
	.byte	0x49
	.byte	0xc
	.long	0x329
	.byte	0
	.uleb128 0x10
	.long	.LASF46
	.byte	0xc
	.byte	0x4a
	.byte	0xc
	.long	0x3c
	.byte	0xf0
	.byte	0
	.uleb128 0x11
	.long	0xe1
	.long	0x339
	.uleb128 0x12
	.long	0x35
	.byte	0x3b
	.byte	0
	.uleb128 0x2
	.long	.LASF47
	.byte	0xc
	.byte	0x4c
	.byte	0x1b
	.long	0x301
	.uleb128 0x6
	.long	0x339
	.uleb128 0x7
	.byte	0x8
	.long	0x214
	.uleb128 0x11
	.long	0xc4
	.long	0x360
	.uleb128 0x12
	.long	0x35
	.byte	0xf
	.byte	0
	.uleb128 0x11
	.long	0xc4
	.long	0x370
	.uleb128 0x12
	.long	0x35
	.byte	0x1f
	.byte	0
	.uleb128 0x11
	.long	0xe1
	.long	0x380
	.uleb128 0x12
	.long	0x35
	.byte	0x4
	.byte	0
	.uleb128 0x11
	.long	0xc4
	.long	0x390
	.uleb128 0x12
	.long	0x35
	.byte	0x3f
	.byte	0
	.uleb128 0x3
	.byte	0x8
	.byte	0x4
	.long	.LASF48
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
	.long	0x3bf
	.uleb128 0x12
	.long	0x35
	.byte	0x3
	.byte	0
	.uleb128 0x13
	.long	.LASF117
	.byte	0x2
	.byte	0x28
	.byte	0x11
	.long	0x3af
	.uleb128 0x3
	.byte	0x10
	.byte	0x5
	.long	.LASF49
	.uleb128 0x3
	.byte	0x10
	.byte	0x7
	.long	.LASF50
	.uleb128 0x9
	.long	.LASF51
	.byte	0x3
	.value	0x12b
	.byte	0x12
	.long	0xf2
	.uleb128 0x3
	.byte	0x1
	.byte	0x2
	.long	.LASF52
	.uleb128 0x7
	.byte	0x8
	.long	0x345
	.uleb128 0x7
	.byte	0x8
	.long	0x339
	.uleb128 0x14
	.long	0x79
	.long	0x417
	.uleb128 0x15
	.long	0x417
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x79
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x123
	.uleb128 0x7
	.byte	0x8
	.long	0x3f9
	.uleb128 0x14
	.long	0x79
	.long	0x441
	.uleb128 0x15
	.long	0x417
	.uleb128 0x15
	.long	0x3a3
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x423
	.uleb128 0x16
	.long	0x452
	.uleb128 0x15
	.long	0x417
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x447
	.uleb128 0x14
	.long	0x79
	.long	0x476
	.uleb128 0x15
	.long	0x417
	.uleb128 0x15
	.long	0x79
	.uleb128 0x15
	.long	0x79
	.uleb128 0x15
	.long	0x43
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x458
	.uleb128 0x17
	.byte	0xe
	.byte	0x1
	.byte	0x27
	.byte	0x3
	.long	0x49e
	.uleb128 0x18
	.long	.LASF53
	.byte	0x1
	.byte	0x28
	.byte	0xe
	.long	0xd5
	.uleb128 0x18
	.long	.LASF54
	.byte	0x1
	.byte	0x33
	.byte	0xd
	.long	0x49e
	.byte	0
	.uleb128 0x11
	.long	0xc4
	.long	0x4ae
	.uleb128 0x12
	.long	0x35
	.byte	0xc
	.byte	0
	.uleb128 0x19
	.value	0x270
	.byte	0x1
	.byte	0x1d
	.byte	0x9
	.long	0x517
	.uleb128 0xf
	.string	"ks"
	.byte	0x1
	.byte	0x1e
	.byte	0xb
	.long	0x339
	.byte	0
	.uleb128 0x10
	.long	.LASF55
	.byte	0x1
	.byte	0x23
	.byte	0xb
	.long	0x2a9
	.byte	0xf4
	.uleb128 0x1a
	.long	.LASF56
	.byte	0x1
	.byte	0x23
	.byte	0x11
	.long	0x2a9
	.value	0x154
	.uleb128 0x1b
	.string	"md"
	.byte	0x1
	.byte	0x23
	.byte	0x17
	.long	0x2a9
	.value	0x1b4
	.uleb128 0x1a
	.long	.LASF57
	.byte	0x1
	.byte	0x26
	.byte	0xa
	.long	0x29
	.value	0x218
	.uleb128 0x1b
	.string	"aux"
	.byte	0x1
	.byte	0x34
	.byte	0x5
	.long	0x47c
	.value	0x220
	.uleb128 0x1a
	.long	.LASF58
	.byte	0x1
	.byte	0x36
	.byte	0xb
	.long	0x380
	.value	0x22e
	.byte	0
	.uleb128 0x2
	.long	.LASF59
	.byte	0x1
	.byte	0x37
	.byte	0x3
	.long	0x4ae
	.uleb128 0x1c
	.long	.LASF60
	.byte	0x1
	.value	0x170
	.byte	0x19
	.long	0x214
	.uleb128 0x9
	.byte	0x3
	.quad	aesni_128_cbc_hmac_sha1_cipher
	.uleb128 0x1c
	.long	.LASF61
	.byte	0x1
	.value	0x17c
	.byte	0x19
	.long	0x214
	.uleb128 0x9
	.byte	0x3
	.quad	aesni_256_cbc_hmac_sha1_cipher
	.uleb128 0x1d
	.long	.LASF62
	.byte	0xd
	.byte	0x3d
	.byte	0xe
	.long	0x43
	.long	0x571
	.uleb128 0x15
	.long	0x43
	.uleb128 0x15
	.long	0x79
	.uleb128 0x15
	.long	0x35
	.byte	0
	.uleb128 0x1e
	.long	.LASF65
	.byte	0xe
	.byte	0x6d
	.byte	0x15
	.long	0x588
	.uleb128 0x15
	.long	0x43
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x1d
	.long	.LASF63
	.byte	0xd
	.byte	0x2b
	.byte	0xe
	.long	0x43
	.long	0x5a8
	.uleb128 0x15
	.long	0x43
	.uleb128 0x15
	.long	0x105
	.uleb128 0x15
	.long	0x35
	.byte	0
	.uleb128 0x1d
	.long	.LASF64
	.byte	0xe
	.byte	0x74
	.byte	0x14
	.long	0x79
	.long	0x5c8
	.uleb128 0x15
	.long	0x105
	.uleb128 0x15
	.long	0x105
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x1e
	.long	.LASF66
	.byte	0xf
	.byte	0x6a
	.byte	0x6
	.long	0x5ee
	.uleb128 0x15
	.long	0x3a3
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x1f
	.long	.LASF67
	.byte	0x10
	.byte	0x43
	.byte	0xd
	.long	0x60f
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
	.long	.LASF68
	.byte	0xf
	.byte	0x97
	.byte	0x5
	.long	0x79
	.long	0x64d
	.uleb128 0x15
	.long	0x397
	.uleb128 0x15
	.long	0x3a3
	.uleb128 0x15
	.long	0x3a9
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x3c
	.byte	0
	.uleb128 0x20
	.long	.LASF118
	.byte	0x12
	.byte	0x53
	.byte	0x1e
	.long	0x397
	.uleb128 0x1d
	.long	.LASF69
	.byte	0xf
	.byte	0x5e
	.byte	0x5
	.long	0x79
	.long	0x688
	.uleb128 0x15
	.long	0x688
	.uleb128 0x15
	.long	0x3a9
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x3d9
	.uleb128 0x1e
	.long	.LASF70
	.byte	0x4
	.byte	0x66
	.byte	0x6
	.long	0x6b9
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x3a3
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x3ed
	.uleb128 0x15
	.long	0x3a3
	.uleb128 0x15
	.long	0x80
	.byte	0
	.uleb128 0x1d
	.long	.LASF71
	.byte	0xb
	.byte	0x55
	.byte	0x14
	.long	0x79
	.long	0x6d4
	.uleb128 0x15
	.long	0x3a3
	.uleb128 0x15
	.long	0x6d4
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x2a9
	.uleb128 0x1e
	.long	.LASF72
	.byte	0x1
	.byte	0x39
	.byte	0x6
	.long	0x70a
	.uleb128 0x15
	.long	0x105
	.uleb128 0x15
	.long	0x43
	.uleb128 0x15
	.long	0x29
	.uleb128 0x15
	.long	0x3ed
	.uleb128 0x15
	.long	0x3a3
	.uleb128 0x15
	.long	0x6d4
	.uleb128 0x15
	.long	0x105
	.byte	0
	.uleb128 0x1d
	.long	.LASF73
	.byte	0xb
	.byte	0x50
	.byte	0x14
	.long	0x79
	.long	0x72a
	.uleb128 0x15
	.long	0x6d4
	.uleb128 0x15
	.long	0x105
	.uleb128 0x15
	.long	0x29
	.byte	0
	.uleb128 0x21
	.long	.LASF74
	.byte	0x9
	.value	0x100
	.byte	0x14
	.long	0x79
	.long	0x741
	.uleb128 0x15
	.long	0x741
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x130
	.uleb128 0x22
	.long	.LASF75
	.byte	0x11
	.value	0x1d0
	.byte	0x15
	.long	0x76e
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
	.long	.LASF76
	.byte	0xb
	.byte	0x4d
	.byte	0x14
	.long	0x79
	.long	0x784
	.uleb128 0x15
	.long	0x6d4
	.byte	0
	.uleb128 0x1d
	.long	.LASF77
	.byte	0x4
	.byte	0x62
	.byte	0x5
	.long	0x79
	.long	0x7a4
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x80
	.uleb128 0x15
	.long	0x3f3
	.byte	0
	.uleb128 0x1d
	.long	.LASF78
	.byte	0x4
	.byte	0x60
	.byte	0x5
	.long	0x79
	.long	0x7c4
	.uleb128 0x15
	.long	0x39d
	.uleb128 0x15
	.long	0x80
	.uleb128 0x15
	.long	0x3f3
	.byte	0
	.uleb128 0x21
	.long	.LASF79
	.byte	0x9
	.value	0x109
	.byte	0x19
	.long	0x3c
	.long	0x7db
	.uleb128 0x15
	.long	0x741
	.byte	0
	.uleb128 0x23
	.long	.LASF80
	.byte	0x1
	.value	0x18c
	.byte	0x13
	.long	0x34a
	.quad	.LFB296
	.quad	.LFE296-.LFB296
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x23
	.long	.LASF81
	.byte	0x1
	.value	0x188
	.byte	0x13
	.long	0x34a
	.quad	.LFB295
	.quad	.LFE295-.LFB295
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x24
	.long	.LASF84
	.byte	0x1
	.value	0x11d
	.byte	0xc
	.long	0x79
	.quad	.LFB294
	.quad	.LFE294-.LFB294
	.uleb128 0x1
	.byte	0x9c
	.long	0x94f
	.uleb128 0x25
	.string	"ctx"
	.byte	0x1
	.value	0x11d
	.byte	0x35
	.long	0x417
	.uleb128 0x2
	.byte	0x77
	.sleb128 24
	.uleb128 0x26
	.long	.LASF82
	.byte	0x1
	.value	0x11d
	.byte	0x3e
	.long	0x79
	.uleb128 0x2
	.byte	0x77
	.sleb128 20
	.uleb128 0x25
	.string	"arg"
	.byte	0x1
	.value	0x11d
	.byte	0x48
	.long	0x79
	.uleb128 0x2
	.byte	0x77
	.sleb128 16
	.uleb128 0x25
	.string	"ptr"
	.byte	0x1
	.value	0x11e
	.byte	0x2b
	.long	0x43
	.uleb128 0x2
	.byte	0x77
	.sleb128 8
	.uleb128 0x27
	.string	"key"
	.byte	0x1
	.value	0x11f
	.byte	0x16
	.long	0x94f
	.uleb128 0x3
	.byte	0x77
	.sleb128 112
	.uleb128 0x28
	.quad	.LBB5
	.quad	.LBE5-.LBB5
	.long	0x90a
	.uleb128 0x1c
	.long	.LASF58
	.byte	0x1
	.value	0x127
	.byte	0xf
	.long	0x380
	.uleb128 0x2
	.byte	0x77
	.sleb128 32
	.uleb128 0x1c
	.long	.LASF83
	.byte	0x1
	.value	0x129
	.byte	0xe
	.long	0x29
	.uleb128 0x3
	.byte	0x77
	.sleb128 104
	.uleb128 0x28
	.quad	.LBB6
	.quad	.LBE6-.LBB6
	.long	0x8e8
	.uleb128 0x27
	.string	"i"
	.byte	0x1
	.value	0x133
	.byte	0x13
	.long	0x29
	.uleb128 0x3
	.byte	0x77
	.sleb128 136
	.byte	0
	.uleb128 0x29
	.quad	.LBB7
	.quad	.LBE7-.LBB7
	.uleb128 0x27
	.string	"i"
	.byte	0x1
	.value	0x139
	.byte	0x13
	.long	0x29
	.uleb128 0x3
	.byte	0x77
	.sleb128 128
	.byte	0
	.byte	0
	.uleb128 0x29
	.quad	.LBB8
	.quad	.LBE8-.LBB8
	.uleb128 0x27
	.string	"p"
	.byte	0x1
	.value	0x14a
	.byte	0x10
	.long	0x3a3
	.uleb128 0x3
	.byte	0x77
	.sleb128 96
	.uleb128 0x29
	.quad	.LBB9
	.quad	.LBE9-.LBB9
	.uleb128 0x27
	.string	"len"
	.byte	0x1
	.value	0x151
	.byte	0x12
	.long	0xd5
	.uleb128 0x3
	.byte	0x77
	.sleb128 126
	.byte	0
	.byte	0
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x517
	.uleb128 0x2a
	.long	.LASF85
	.byte	0x1
	.byte	0x69
	.byte	0xc
	.long	0x79
	.quad	.LFB293
	.quad	.LFE293-.LFB293
	.uleb128 0x1
	.byte	0x9c
	.long	0xaef
	.uleb128 0x2b
	.string	"ctx"
	.byte	0x1
	.byte	0x69
	.byte	0x37
	.long	0x417
	.uleb128 0x3
	.byte	0x76
	.sleb128 -280
	.uleb128 0x2b
	.string	"out"
	.byte	0x1
	.byte	0x69
	.byte	0x45
	.long	0x3a3
	.uleb128 0x3
	.byte	0x76
	.sleb128 -288
	.uleb128 0x2b
	.string	"in"
	.byte	0x1
	.byte	0x6a
	.byte	0x36
	.long	0x39d
	.uleb128 0x3
	.byte	0x76
	.sleb128 -296
	.uleb128 0x2b
	.string	"len"
	.byte	0x1
	.byte	0x6a
	.byte	0x41
	.long	0x29
	.uleb128 0x3
	.byte	0x76
	.sleb128 -304
	.uleb128 0x2c
	.string	"key"
	.byte	0x1
	.byte	0x6b
	.byte	0x16
	.long	0x94f
	.uleb128 0x3
	.byte	0x76
	.sleb128 -72
	.uleb128 0x2d
	.long	.LASF86
	.byte	0x1
	.byte	0x6c
	.byte	0xa
	.long	0x29
	.uleb128 0x2
	.byte	0x76
	.sleb128 -40
	.uleb128 0x2d
	.long	.LASF36
	.byte	0x1
	.byte	0x6c
	.byte	0x26
	.long	0x29
	.uleb128 0x2
	.byte	0x76
	.sleb128 -48
	.uleb128 0x2e
	.long	.LASF119
	.long	0xaff
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.0
	.uleb128 0x28
	.quad	.LBB2
	.quad	.LBE2-.LBB2
	.long	0xa5b
	.uleb128 0x2d
	.long	.LASF87
	.byte	0x1
	.byte	0x8e
	.byte	0xc
	.long	0x29
	.uleb128 0x2
	.byte	0x76
	.sleb128 -56
	.uleb128 0x2d
	.long	.LASF88
	.byte	0x1
	.byte	0x8f
	.byte	0xc
	.long	0x29
	.uleb128 0x2
	.byte	0x76
	.sleb128 -64
	.uleb128 0x2d
	.long	.LASF89
	.byte	0x1
	.byte	0x90
	.byte	0xc
	.long	0x29
	.uleb128 0x3
	.byte	0x76
	.sleb128 -104
	.uleb128 0x29
	.quad	.LBB3
	.quad	.LBE3-.LBB3
	.uleb128 0x2c
	.string	"l"
	.byte	0x1
	.byte	0xc5
	.byte	0x17
	.long	0x3c
	.uleb128 0x3
	.byte	0x76
	.sleb128 -108
	.byte	0
	.byte	0
	.uleb128 0x29
	.quad	.LBB4
	.quad	.LBE4-.LBB4
	.uleb128 0x2d
	.long	.LASF90
	.byte	0x1
	.byte	0xeb
	.byte	0xc
	.long	0x29
	.uleb128 0x3
	.byte	0x76
	.sleb128 -120
	.uleb128 0x2d
	.long	.LASF91
	.byte	0x1
	.byte	0xec
	.byte	0x13
	.long	0x3d9
	.uleb128 0x3
	.byte	0x76
	.sleb128 -128
	.uleb128 0x2d
	.long	.LASF92
	.byte	0x1
	.byte	0xf4
	.byte	0xc
	.long	0x29
	.uleb128 0x3
	.byte	0x76
	.sleb128 -80
	.uleb128 0x2c
	.string	"mac"
	.byte	0x1
	.byte	0xfa
	.byte	0xd
	.long	0x380
	.uleb128 0x3
	.byte	0x76
	.sleb128 -272
	.uleb128 0x2d
	.long	.LASF93
	.byte	0x1
	.byte	0xfb
	.byte	0xc
	.long	0x29
	.uleb128 0x3
	.byte	0x76
	.sleb128 -136
	.uleb128 0x2d
	.long	.LASF94
	.byte	0x1
	.byte	0xfc
	.byte	0xd
	.long	0x380
	.uleb128 0x3
	.byte	0x76
	.sleb128 -208
	.uleb128 0x2d
	.long	.LASF95
	.byte	0x1
	.byte	0xfd
	.byte	0xe
	.long	0x3a3
	.uleb128 0x3
	.byte	0x76
	.sleb128 -88
	.uleb128 0x1c
	.long	.LASF96
	.byte	0x1
	.value	0x10c
	.byte	0x13
	.long	0x3d9
	.uleb128 0x3
	.byte	0x76
	.sleb128 -96
	.byte	0
	.byte	0
	.uleb128 0x11
	.long	0xab
	.long	0xaff
	.uleb128 0x12
	.long	0x35
	.byte	0x1a
	.byte	0
	.uleb128 0x6
	.long	0xaef
	.uleb128 0x2a
	.long	.LASF97
	.byte	0x1
	.byte	0x3d
	.byte	0xc
	.long	0x79
	.quad	.LFB292
	.quad	.LFE292-.LFB292
	.uleb128 0x1
	.byte	0x9c
	.long	0xb8f
	.uleb128 0x2b
	.string	"ctx"
	.byte	0x1
	.byte	0x3d
	.byte	0x39
	.long	0x417
	.uleb128 0x2
	.byte	0x77
	.sleb128 24
	.uleb128 0x2f
	.long	.LASF98
	.byte	0x1
	.byte	0x3e
	.byte	0x38
	.long	0x39d
	.uleb128 0x2
	.byte	0x77
	.sleb128 16
	.uleb128 0x2b
	.string	"iv"
	.byte	0x1
	.byte	0x3f
	.byte	0x38
	.long	0x39d
	.uleb128 0x2
	.byte	0x77
	.sleb128 8
	.uleb128 0x2b
	.string	"enc"
	.byte	0x1
	.byte	0x3f
	.byte	0x40
	.long	0x79
	.uleb128 0x2
	.byte	0x77
	.sleb128 4
	.uleb128 0x2c
	.string	"key"
	.byte	0x1
	.byte	0x40
	.byte	0x16
	.long	0x94f
	.uleb128 0x2
	.byte	0x77
	.sleb128 48
	.uleb128 0x2c
	.string	"ret"
	.byte	0x1
	.byte	0x41
	.byte	0x7
	.long	0x79
	.uleb128 0x2
	.byte	0x77
	.sleb128 60
	.uleb128 0x2d
	.long	.LASF99
	.byte	0x1
	.byte	0x43
	.byte	0x7
	.long	0x79
	.uleb128 0x2
	.byte	0x77
	.sleb128 44
	.byte	0
	.uleb128 0x30
	.long	.LASF106
	.byte	0x4
	.byte	0x2d
	.byte	0x14
	.long	0x79
	.quad	.LFB282
	.quad	.LFE282-.LFB282
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x24
	.long	.LASF100
	.byte	0x3
	.value	0x3cc
	.byte	0x15
	.long	0x43
	.quad	.LFB249
	.quad	.LFE249-.LFB249
	.uleb128 0x1
	.byte	0x9c
	.long	0xbfd
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
	.long	.LASF101
	.byte	0x3
	.value	0x3bc
	.byte	0x15
	.long	0x43
	.quad	.LFB247
	.quad	.LFE247-.LFB247
	.uleb128 0x1
	.byte	0x9c
	.long	0xc4f
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
	.long	.LASF102
	.byte	0x3
	.value	0x1b5
	.byte	0x1d
	.long	0x3d9
	.quad	.LFB230
	.quad	.LFE230-.LFB230
	.uleb128 0x1
	.byte	0x9c
	.long	0xc8f
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
	.long	.LASF103
	.byte	0x3
	.value	0x1a8
	.byte	0x1d
	.long	0x3d9
	.quad	.LFB228
	.quad	.LFE228-.LFB228
	.uleb128 0x1
	.byte	0x9c
	.long	0xccf
	.uleb128 0x25
	.string	"a"
	.byte	0x3
	.value	0x1a8
	.byte	0x3e
	.long	0x3d9
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x25
	.string	"b"
	.byte	0x3
	.value	0x1a9
	.byte	0x3e
	.long	0x3d9
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x24
	.long	.LASF104
	.byte	0x3
	.value	0x192
	.byte	0x1d
	.long	0x3d9
	.quad	.LFB226
	.quad	.LFE226-.LFB226
	.uleb128 0x1
	.byte	0x9c
	.long	0xd01
	.uleb128 0x25
	.string	"a"
	.byte	0x3
	.value	0x192
	.byte	0x43
	.long	0x3d9
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x31
	.long	.LASF105
	.byte	0x3
	.value	0x157
	.byte	0x1d
	.long	0x3d9
	.quad	.LFB221
	.quad	.LFE221-.LFB221
	.uleb128 0x1
	.byte	0x9c
	.long	0xd33
	.uleb128 0x25
	.string	"a"
	.byte	0x3
	.value	0x157
	.byte	0x3f
	.long	0x3d9
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x30
	.long	.LASF107
	.byte	0x2
	.byte	0x77
	.byte	0x14
	.long	0x79
	.quad	.LFB209
	.quad	.LFE209-.LFB209
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x30
	.long	.LASF108
	.byte	0x2
	.byte	0x61
	.byte	0x14
	.long	0x79
	.quad	.LFB204
	.quad	.LFE204-.LFB204
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x30
	.long	.LASF109
	.byte	0x2
	.byte	0x59
	.byte	0x14
	.long	0x79
	.quad	.LFB202
	.quad	.LFE202-.LFB202
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x30
	.long	.LASF110
	.byte	0x2
	.byte	0x52
	.byte	0x14
	.long	0x79
	.quad	.LFB201
	.quad	.LFE201-.LFB201
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x30
	.long	.LASF111
	.byte	0x2
	.byte	0x3b
	.byte	0x14
	.long	0x79
	.quad	.LFB196
	.quad	.LFE196-.LFB196
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x32
	.long	.LASF112
	.byte	0x2
	.byte	0x30
	.byte	0x20
	.long	0xde7
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
	.quad	.LFB202
	.quad	.LFE202-.LFB202
	.quad	.LFB204
	.quad	.LFE204-.LFB204
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
	.quad	.LFB202
	.quad	.LFE202
	.quad	.LFB204
	.quad	.LFE204
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
.LASF117:
	.string	"aws_lc_0_38_0_OPENSSL_ia32cap_P"
.LASF5:
	.string	"size_t"
.LASF118:
	.string	"aws_lc_0_38_0_EVP_sha1"
.LASF44:
	.string	"aes_key_st"
.LASF18:
	.string	"uint64_t"
.LASF6:
	.string	"__uint8_t"
.LASF82:
	.string	"type"
.LASF109:
	.string	"CRYPTO_is_AVX_capable"
.LASF47:
	.string	"AES_KEY"
.LASF30:
	.string	"final"
.LASF19:
	.string	"long long unsigned int"
.LASF53:
	.string	"tls_ver"
.LASF21:
	.string	"EVP_CIPHER_CTX"
.LASF110:
	.string	"CRYPTO_is_AESNI_capable"
.LASF32:
	.string	"EVP_CIPHER"
.LASF116:
	.string	"env_md_st"
.LASF86:
	.string	"plen"
.LASF13:
	.string	"long long int"
.LASF4:
	.string	"signed char"
.LASF112:
	.string	"OPENSSL_ia32cap_get"
.LASF28:
	.string	"buf_len"
.LASF119:
	.string	"__PRETTY_FUNCTION__"
.LASF68:
	.string	"aws_lc_0_38_0_EVP_tls_cbc_digest_record"
.LASF71:
	.string	"aws_lc_0_38_0_SHA1_Final"
.LASF36:
	.string	"iv_len"
.LASF83:
	.string	"u_arg"
.LASF85:
	.string	"aesni_cbc_hmac_sha1_cipher"
.LASF73:
	.string	"aws_lc_0_38_0_SHA1_Update"
.LASF10:
	.string	"long int"
.LASF16:
	.string	"uint16_t"
.LASF48:
	.string	"double"
.LASF108:
	.string	"CRYPTO_is_AMD_XOP_support"
.LASF97:
	.string	"aesni_cbc_hmac_sha1_init_key"
.LASF46:
	.string	"rounds"
.LASF25:
	.string	"key_len"
.LASF76:
	.string	"aws_lc_0_38_0_SHA1_Init"
.LASF41:
	.string	"SHA_CTX"
.LASF9:
	.string	"__uint32_t"
.LASF105:
	.string	"constant_time_msb_w"
.LASF58:
	.string	"hmac_key"
.LASF103:
	.string	"constant_time_eq_w"
.LASF66:
	.string	"aws_lc_0_38_0_EVP_tls_cbc_copy_mac"
.LASF1:
	.string	"unsigned int"
.LASF54:
	.string	"tls_aad"
.LASF95:
	.string	"record_mac"
.LASF49:
	.string	"__int128"
.LASF0:
	.string	"long unsigned int"
.LASF106:
	.string	"hwaes_capable"
.LASF26:
	.string	"encrypt"
.LASF79:
	.string	"aws_lc_0_38_0_EVP_CIPHER_CTX_key_length"
.LASF43:
	.string	"data"
.LASF3:
	.string	"short unsigned int"
.LASF59:
	.string	"EVP_AES_HMAC_SHA1"
.LASF61:
	.string	"aesni_256_cbc_hmac_sha1_cipher"
.LASF89:
	.string	"blocks"
.LASF70:
	.string	"aws_lc_0_38_0_aes_hw_cbc_encrypt"
.LASF7:
	.string	"short int"
.LASF94:
	.string	"record_mac_tmp"
.LASF45:
	.string	"rd_key"
.LASF35:
	.string	"block_size"
.LASF11:
	.string	"__uint64_t"
.LASF84:
	.string	"aesni_cbc_hmac_sha1_ctrl"
.LASF20:
	.string	"EVP_MD"
.LASF56:
	.string	"tail"
.LASF34:
	.string	"evp_cipher_st"
.LASF98:
	.string	"inkey"
.LASF81:
	.string	"aws_lc_0_38_0_EVP_aes_128_cbc_hmac_sha1"
.LASF111:
	.string	"CRYPTO_is_intel_cpu"
.LASF42:
	.string	"sha_state_st"
.LASF72:
	.string	"aws_lc_0_38_0_aesni_cbc_sha1_enc"
.LASF39:
	.string	"cleanup"
.LASF57:
	.string	"payload_length"
.LASF102:
	.string	"constant_time_eq_int"
.LASF38:
	.string	"init"
.LASF69:
	.string	"aws_lc_0_38_0_EVP_tls_cbc_remove_padding"
.LASF75:
	.string	"aws_lc_0_38_0_ERR_put_error"
.LASF67:
	.string	"__assert_fail"
.LASF50:
	.string	"__int128 unsigned"
.LASF52:
	.string	"_Bool"
.LASF2:
	.string	"unsigned char"
.LASF115:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF23:
	.string	"app_data"
.LASF93:
	.string	"mac_len"
.LASF113:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF92:
	.string	"data_len"
.LASF37:
	.string	"ctx_size"
.LASF51:
	.string	"crypto_word_t"
.LASF78:
	.string	"aws_lc_0_38_0_aes_hw_set_encrypt_key"
.LASF55:
	.string	"head"
.LASF90:
	.string	"data_plus_mac_len"
.LASF14:
	.string	"long double"
.LASF12:
	.string	"char"
.LASF100:
	.string	"OPENSSL_memset"
.LASF114:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aes_cbc_hmac_sha1.c"
.LASF24:
	.string	"cipher_data"
.LASF8:
	.string	"__uint16_t"
.LASF107:
	.string	"CRYPTO_is_SHAEXT_capable"
.LASF99:
	.string	"key_bits"
.LASF31:
	.string	"poisoned"
.LASF40:
	.string	"ctrl"
.LASF63:
	.string	"memcpy"
.LASF74:
	.string	"aws_lc_0_38_0_EVP_CIPHER_CTX_encrypting"
.LASF88:
	.string	"sha_off"
.LASF17:
	.string	"uint32_t"
.LASF62:
	.string	"memset"
.LASF96:
	.string	"good"
.LASF65:
	.string	"aws_lc_0_38_0_OPENSSL_cleanse"
.LASF15:
	.string	"uint8_t"
.LASF27:
	.string	"flags"
.LASF22:
	.string	"cipher"
.LASF80:
	.string	"aws_lc_0_38_0_EVP_aes_256_cbc_hmac_sha1"
.LASF87:
	.string	"aes_off"
.LASF101:
	.string	"OPENSSL_memcpy"
.LASF60:
	.string	"aesni_128_cbc_hmac_sha1_cipher"
.LASF77:
	.string	"aws_lc_0_38_0_aes_hw_set_decrypt_key"
.LASF29:
	.string	"final_used"
.LASF33:
	.string	"evp_cipher_ctx_st"
.LASF104:
	.string	"constant_time_is_zero_w"
.LASF64:
	.string	"aws_lc_0_38_0_CRYPTO_memcmp"
.LASF91:
	.string	"padding_ok"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
