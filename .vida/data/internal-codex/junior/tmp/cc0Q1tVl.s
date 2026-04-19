	.file	"chacha.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/chacha/chacha.c"
	.section	.text.buffers_alias,"ax",@progbits
	.type	buffers_alias, @function
buffers_alias:
.LFB63:
	.file 2 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/chacha/../internal.h"
	.loc 2 243 65
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	movq	%rdx, -40(%rbp)
	movq	%rcx, -48(%rbp)
	.loc 2 248 13
	movq	-24(%rbp), %rax
	movq	%rax, -8(%rbp)
	.loc 2 249 13
	movq	-40(%rbp), %rax
	movq	%rax, -16(%rbp)
	.loc 2 250 14
	movq	-8(%rbp), %rdx
	movq	-32(%rbp), %rax
	addq	%rdx, %rax
	.loc 2 250 28
	cmpq	%rax, -16(%rbp)
	jnb	.L2
	.loc 2 250 35 discriminator 1
	movq	-16(%rbp), %rdx
	movq	-48(%rbp), %rax
	addq	%rdx, %rax
	.loc 2 250 28 discriminator 1
	cmpq	%rax, -8(%rbp)
	jnb	.L2
	.loc 2 250 28 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 2 250 28
	jmp	.L4
.L2:
	.loc 2 250 28 discriminator 4
	movl	$0, %eax
.L4:
	.loc 2 251 1 is_stmt 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE63:
	.size	buffers_alias, .-buffers_alias
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
	jne	.L6
	.loc 2 958 12
	movq	-8(%rbp), %rax
	jmp	.L7
.L6:
	.loc 2 961 10
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	memcpy@PLT
.L7:
	.loc 2 962 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE94:
	.size	OPENSSL_memcpy, .-OPENSSL_memcpy
	.section	.text.CRYPTO_load_u32_le,"ax",@progbits
	.type	CRYPTO_load_u32_le, @function
CRYPTO_load_u32_le:
.LFB101:
	.loc 2 1021 59
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	.loc 2 1023 3
	movq	-24(%rbp), %rcx
	leaq	-4(%rbp), %rax
	movl	$4, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 2 1027 10
	movl	-4(%rbp), %eax
	.loc 2 1029 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE101:
	.size	CRYPTO_load_u32_le, .-CRYPTO_load_u32_le
	.section	.text.CRYPTO_rotl_u32,"ax",@progbits
	.type	CRYPTO_rotl_u32, @function
CRYPTO_rotl_u32:
.LFB112:
	.loc 2 1138 67
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movl	%edi, -4(%rbp)
	movl	%esi, -8(%rbp)
	.loc 2 1142 27
	movl	-8(%rbp), %eax
	movl	-4(%rbp), %edx
	movl	%eax, %ecx
	roll	%cl, %edx
	movl	%edx, %eax
	.loc 2 1144 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE112:
	.size	CRYPTO_rotl_u32, .-CRYPTO_rotl_u32
	.section	.text.OPENSSL_ia32cap_get,"ax",@progbits
	.type	OPENSSL_ia32cap_get, @function
OPENSSL_ia32cap_get:
.LFB128:
	.file 3 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/chacha/../fipsmodule/cpucap/internal.h"
	.loc 3 48 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 3 49 10
	movq	aws_lc_0_38_0_OPENSSL_ia32cap_P@GOTPCREL(%rip), %rax
	.loc 3 50 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE128:
	.size	OPENSSL_ia32cap_get, .-OPENSSL_ia32cap_get
	.section	.text.CRYPTO_is_SSSE3_capable,"ax",@progbits
	.type	CRYPTO_is_SSSE3_capable, @function
CRYPTO_is_SSSE3_capable:
.LFB132:
	.loc 3 70 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 3 71 11
	call	OPENSSL_ia32cap_get
	.loc 3 71 32 discriminator 1
	addq	$4, %rax
	movl	(%rax), %eax
	.loc 3 71 36 discriminator 1
	andl	$512, %eax
	.loc 3 71 48 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 3 72 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE132:
	.size	CRYPTO_is_SSSE3_capable, .-CRYPTO_is_SSSE3_capable
	.section	.text.CRYPTO_is_MOVBE_capable,"ax",@progbits
	.type	CRYPTO_is_MOVBE_capable, @function
CRYPTO_is_MOVBE_capable:
.LFB134:
	.loc 3 78 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 3 79 11
	call	OPENSSL_ia32cap_get
	.loc 3 79 32 discriminator 1
	addq	$4, %rax
	movl	(%rax), %eax
	.loc 3 79 36 discriminator 1
	andl	$4194304, %eax
	.loc 3 79 49 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 3 80 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE134:
	.size	CRYPTO_is_MOVBE_capable, .-CRYPTO_is_MOVBE_capable
	.section	.text.CRYPTO_is_AVX2_capable,"ax",@progbits
	.type	CRYPTO_is_AVX2_capable, @function
CRYPTO_is_AVX2_capable:
.LFB140:
	.loc 3 107 49
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 3 108 11
	call	OPENSSL_ia32cap_get
	.loc 3 108 32 discriminator 1
	addq	$8, %rax
	movl	(%rax), %eax
	.loc 3 108 36 discriminator 1
	andl	$32, %eax
	.loc 3 108 48 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 3 109 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE140:
	.size	CRYPTO_is_AVX2_capable, .-CRYPTO_is_AVX2_capable
	.section	.text.CRYPTO_cpu_perf_is_like_silvermont,"ax",@progbits
	.type	CRYPTO_cpu_perf_is_like_silvermont, @function
CRYPTO_cpu_perf_is_like_silvermont:
.LFB149:
	.loc 3 163 61
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	.loc 3 172 34
	call	OPENSSL_ia32cap_get
	.loc 3 172 55 discriminator 1
	addq	$4, %rax
	movl	(%rax), %eax
	.loc 3 172 59 discriminator 1
	andl	$67108864, %eax
	.loc 3 172 73 discriminator 1
	testl	%eax, %eax
	setne	%al
	.loc 3 172 7 discriminator 1
	movzbl	%al, %eax
	movl	%eax, -4(%rbp)
	.loc 3 173 35
	cmpl	$0, -4(%rbp)
	jne	.L21
	.loc 3 173 38 discriminator 1
	call	CRYPTO_is_MOVBE_capable
	.loc 3 173 35 discriminator 1
	testl	%eax, %eax
	je	.L21
	.loc 3 173 35 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 3 173 35
	jmp	.L23
.L21:
	.loc 3 173 35 discriminator 4
	movl	$0, %eax
.L23:
	.loc 3 174 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE149:
	.size	CRYPTO_cpu_perf_is_like_silvermont, .-CRYPTO_cpu_perf_is_like_silvermont
	.section	.text.ChaCha20_ctr32_avx2_capable,"ax",@progbits
	.type	ChaCha20_ctr32_avx2_capable, @function
ChaCha20_ctr32_avx2_capable:
.LFB150:
	.file 4 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/chacha/internal.h"
	.loc 4 61 60
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$8, %rsp
	movq	%rdi, -8(%rbp)
	.loc 4 62 20
	cmpq	$128, -8(%rbp)
	jbe	.L25
	.loc 4 62 23 discriminator 1
	call	CRYPTO_is_AVX2_capable
	.loc 4 62 20 discriminator 1
	testl	%eax, %eax
	je	.L25
	.loc 4 62 20 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 4 62 20
	jmp	.L27
.L25:
	.loc 4 62 20 discriminator 4
	movl	$0, %eax
.L27:
	.loc 4 63 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE150:
	.size	ChaCha20_ctr32_avx2_capable, .-ChaCha20_ctr32_avx2_capable
	.section	.text.ChaCha20_ctr32_ssse3_4x_capable,"ax",@progbits
	.type	ChaCha20_ctr32_ssse3_4x_capable, @function
ChaCha20_ctr32_ssse3_4x_capable:
.LFB151:
	.loc 4 68 64
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$24, %rsp
	movq	%rdi, -24(%rbp)
	.loc 4 69 27
	cmpq	$128, -24(%rbp)
	jbe	.L29
	.loc 4 69 30 discriminator 1
	call	CRYPTO_is_SSSE3_capable
	.loc 4 69 27 discriminator 1
	testl	%eax, %eax
	je	.L29
	.loc 4 69 27 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 4 69 27
	jmp	.L30
.L29:
	.loc 4 69 27 discriminator 4
	movl	$0, %eax
.L30:
	.loc 4 69 7 is_stmt 1 discriminator 6
	movl	%eax, -4(%rbp)
	.loc 4 70 26
	cmpq	$192, -24(%rbp)
	ja	.L31
	.loc 4 70 30 discriminator 2
	call	CRYPTO_cpu_perf_is_like_silvermont
	.loc 4 70 26 discriminator 1
	testl	%eax, %eax
	jne	.L32
.L31:
	.loc 4 70 26 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 4 70 26
	jmp	.L33
.L32:
	.loc 4 70 26 discriminator 4
	movl	$0, %eax
.L33:
	.loc 4 70 7 is_stmt 1 discriminator 6
	movl	%eax, -8(%rbp)
	.loc 4 71 18
	cmpl	$0, -4(%rbp)
	je	.L34
	.loc 4 71 18 is_stmt 0 discriminator 1
	cmpl	$0, -8(%rbp)
	je	.L34
	.loc 4 71 18 discriminator 3
	movl	$1, %eax
	.loc 4 71 18
	jmp	.L36
.L34:
	.loc 4 71 18 discriminator 4
	movl	$0, %eax
.L36:
	.loc 4 72 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE151:
	.size	ChaCha20_ctr32_ssse3_4x_capable, .-ChaCha20_ctr32_ssse3_4x_capable
	.section	.text.ChaCha20_ctr32_ssse3_capable,"ax",@progbits
	.type	ChaCha20_ctr32_ssse3_capable, @function
ChaCha20_ctr32_ssse3_capable:
.LFB152:
	.loc 4 77 61
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$8, %rsp
	movq	%rdi, -8(%rbp)
	.loc 4 78 20
	cmpq	$128, -8(%rbp)
	jbe	.L38
	.loc 4 78 23 discriminator 1
	call	CRYPTO_is_SSSE3_capable
	.loc 4 78 20 discriminator 1
	testl	%eax, %eax
	je	.L38
	.loc 4 78 20 is_stmt 0 discriminator 3
	movl	$1, %eax
	.loc 4 78 20
	jmp	.L40
.L38:
	.loc 4 78 20 discriminator 4
	movl	$0, %eax
.L40:
	.loc 4 79 1 is_stmt 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE152:
	.size	ChaCha20_ctr32_ssse3_capable, .-ChaCha20_ctr32_ssse3_capable
	.section	.rodata.sigma_words,"a"
	.align 16
	.type	sigma_words, @object
	.size	sigma_words, 16
sigma_words:
	.long	1634760805
	.long	857760878
	.long	2036477234
	.long	1797285236
	.section	.text.aws_lc_0_38_0_CRYPTO_hchacha20,"ax",@progbits
	.globl	aws_lc_0_38_0_CRYPTO_hchacha20
	.type	aws_lc_0_38_0_CRYPTO_hchacha20, @function
aws_lc_0_38_0_CRYPTO_hchacha20:
.LFB153:
	.loc 1 47 48
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$112, %rsp
	movq	%rdi, -88(%rbp)
	movq	%rsi, -96(%rbp)
	movq	%rdx, -104(%rbp)
	.loc 1 49 3
	leaq	-80(%rbp), %rax
	movl	$16, %edx
	leaq	sigma_words(%rip), %rcx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 58 3
	movq	-96(%rbp), %rax
	leaq	-80(%rbp), %rdx
	leaq	16(%rdx), %rcx
	movl	$32, %edx
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
	.loc 1 59 3
	movq	-104(%rbp), %rax
	leaq	-80(%rbp), %rdx
	leaq	48(%rdx), %rcx
	movl	$16, %edx
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
.LBB2:
	.loc 1 62 15
	movq	$0, -8(%rbp)
	.loc 1 62 3
	jmp	.L42
.L43:
	.loc 1 63 5
	movl	-80(%rbp), %edx
	movl	-64(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -80(%rbp)
	movl	-32(%rbp), %edx
	movl	-80(%rbp), %eax
	xorl	%edx, %eax
	movl	$16, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 63 5 is_stmt 0 discriminator 1
	movl	%eax, -32(%rbp)
	movl	-48(%rbp), %edx
	movl	-32(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -48(%rbp)
	movl	-64(%rbp), %edx
	movl	-48(%rbp), %eax
	xorl	%edx, %eax
	movl	$12, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 63 5 discriminator 2
	movl	%eax, -64(%rbp)
	movl	-80(%rbp), %edx
	movl	-64(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -80(%rbp)
	movl	-32(%rbp), %edx
	movl	-80(%rbp), %eax
	xorl	%edx, %eax
	movl	$8, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 63 5 discriminator 3
	movl	%eax, -32(%rbp)
	movl	-48(%rbp), %edx
	movl	-32(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -48(%rbp)
	movl	-64(%rbp), %edx
	movl	-48(%rbp), %eax
	xorl	%edx, %eax
	movl	$7, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 63 5 discriminator 4
	movl	%eax, -64(%rbp)
	.loc 1 64 5 is_stmt 1
	movl	-76(%rbp), %edx
	movl	-60(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -76(%rbp)
	movl	-28(%rbp), %edx
	movl	-76(%rbp), %eax
	xorl	%edx, %eax
	movl	$16, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 64 5 is_stmt 0 discriminator 1
	movl	%eax, -28(%rbp)
	movl	-44(%rbp), %edx
	movl	-28(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -44(%rbp)
	movl	-60(%rbp), %edx
	movl	-44(%rbp), %eax
	xorl	%edx, %eax
	movl	$12, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 64 5 discriminator 2
	movl	%eax, -60(%rbp)
	movl	-76(%rbp), %edx
	movl	-60(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -76(%rbp)
	movl	-28(%rbp), %edx
	movl	-76(%rbp), %eax
	xorl	%edx, %eax
	movl	$8, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 64 5 discriminator 3
	movl	%eax, -28(%rbp)
	movl	-44(%rbp), %edx
	movl	-28(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -44(%rbp)
	movl	-60(%rbp), %edx
	movl	-44(%rbp), %eax
	xorl	%edx, %eax
	movl	$7, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 64 5 discriminator 4
	movl	%eax, -60(%rbp)
	.loc 1 65 5 is_stmt 1
	movl	-72(%rbp), %edx
	movl	-56(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -72(%rbp)
	movl	-24(%rbp), %edx
	movl	-72(%rbp), %eax
	xorl	%edx, %eax
	movl	$16, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 65 5 is_stmt 0 discriminator 1
	movl	%eax, -24(%rbp)
	movl	-40(%rbp), %edx
	movl	-24(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -40(%rbp)
	movl	-56(%rbp), %edx
	movl	-40(%rbp), %eax
	xorl	%edx, %eax
	movl	$12, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 65 5 discriminator 2
	movl	%eax, -56(%rbp)
	movl	-72(%rbp), %edx
	movl	-56(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -72(%rbp)
	movl	-24(%rbp), %edx
	movl	-72(%rbp), %eax
	xorl	%edx, %eax
	movl	$8, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 65 5 discriminator 3
	movl	%eax, -24(%rbp)
	movl	-40(%rbp), %edx
	movl	-24(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -40(%rbp)
	movl	-56(%rbp), %edx
	movl	-40(%rbp), %eax
	xorl	%edx, %eax
	movl	$7, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 65 5 discriminator 4
	movl	%eax, -56(%rbp)
	.loc 1 66 5 is_stmt 1
	movl	-68(%rbp), %edx
	movl	-52(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -68(%rbp)
	movl	-20(%rbp), %edx
	movl	-68(%rbp), %eax
	xorl	%edx, %eax
	movl	$16, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 66 5 is_stmt 0 discriminator 1
	movl	%eax, -20(%rbp)
	movl	-36(%rbp), %edx
	movl	-20(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -36(%rbp)
	movl	-52(%rbp), %edx
	movl	-36(%rbp), %eax
	xorl	%edx, %eax
	movl	$12, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 66 5 discriminator 2
	movl	%eax, -52(%rbp)
	movl	-68(%rbp), %edx
	movl	-52(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -68(%rbp)
	movl	-20(%rbp), %edx
	movl	-68(%rbp), %eax
	xorl	%edx, %eax
	movl	$8, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 66 5 discriminator 3
	movl	%eax, -20(%rbp)
	movl	-36(%rbp), %edx
	movl	-20(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -36(%rbp)
	movl	-52(%rbp), %edx
	movl	-36(%rbp), %eax
	xorl	%edx, %eax
	movl	$7, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 66 5 discriminator 4
	movl	%eax, -52(%rbp)
	.loc 1 67 5 is_stmt 1
	movl	-80(%rbp), %edx
	movl	-60(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -80(%rbp)
	movl	-20(%rbp), %edx
	movl	-80(%rbp), %eax
	xorl	%edx, %eax
	movl	$16, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 67 5 is_stmt 0 discriminator 1
	movl	%eax, -20(%rbp)
	movl	-40(%rbp), %edx
	movl	-20(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -40(%rbp)
	movl	-60(%rbp), %edx
	movl	-40(%rbp), %eax
	xorl	%edx, %eax
	movl	$12, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 67 5 discriminator 2
	movl	%eax, -60(%rbp)
	movl	-80(%rbp), %edx
	movl	-60(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -80(%rbp)
	movl	-20(%rbp), %edx
	movl	-80(%rbp), %eax
	xorl	%edx, %eax
	movl	$8, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 67 5 discriminator 3
	movl	%eax, -20(%rbp)
	movl	-40(%rbp), %edx
	movl	-20(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -40(%rbp)
	movl	-60(%rbp), %edx
	movl	-40(%rbp), %eax
	xorl	%edx, %eax
	movl	$7, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 67 5 discriminator 4
	movl	%eax, -60(%rbp)
	.loc 1 68 5 is_stmt 1
	movl	-76(%rbp), %edx
	movl	-56(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -76(%rbp)
	movl	-32(%rbp), %edx
	movl	-76(%rbp), %eax
	xorl	%edx, %eax
	movl	$16, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 68 5 is_stmt 0 discriminator 1
	movl	%eax, -32(%rbp)
	movl	-36(%rbp), %edx
	movl	-32(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -36(%rbp)
	movl	-56(%rbp), %edx
	movl	-36(%rbp), %eax
	xorl	%edx, %eax
	movl	$12, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 68 5 discriminator 2
	movl	%eax, -56(%rbp)
	movl	-76(%rbp), %edx
	movl	-56(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -76(%rbp)
	movl	-32(%rbp), %edx
	movl	-76(%rbp), %eax
	xorl	%edx, %eax
	movl	$8, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 68 5 discriminator 3
	movl	%eax, -32(%rbp)
	movl	-36(%rbp), %edx
	movl	-32(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -36(%rbp)
	movl	-56(%rbp), %edx
	movl	-36(%rbp), %eax
	xorl	%edx, %eax
	movl	$7, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 68 5 discriminator 4
	movl	%eax, -56(%rbp)
	.loc 1 69 5 is_stmt 1
	movl	-72(%rbp), %edx
	movl	-52(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -72(%rbp)
	movl	-28(%rbp), %edx
	movl	-72(%rbp), %eax
	xorl	%edx, %eax
	movl	$16, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 69 5 is_stmt 0 discriminator 1
	movl	%eax, -28(%rbp)
	movl	-48(%rbp), %edx
	movl	-28(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -48(%rbp)
	movl	-52(%rbp), %edx
	movl	-48(%rbp), %eax
	xorl	%edx, %eax
	movl	$12, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 69 5 discriminator 2
	movl	%eax, -52(%rbp)
	movl	-72(%rbp), %edx
	movl	-52(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -72(%rbp)
	movl	-28(%rbp), %edx
	movl	-72(%rbp), %eax
	xorl	%edx, %eax
	movl	$8, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 69 5 discriminator 3
	movl	%eax, -28(%rbp)
	movl	-48(%rbp), %edx
	movl	-28(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -48(%rbp)
	movl	-52(%rbp), %edx
	movl	-48(%rbp), %eax
	xorl	%edx, %eax
	movl	$7, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 69 5 discriminator 4
	movl	%eax, -52(%rbp)
	.loc 1 70 5 is_stmt 1
	movl	-68(%rbp), %edx
	movl	-64(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -68(%rbp)
	movl	-24(%rbp), %edx
	movl	-68(%rbp), %eax
	xorl	%edx, %eax
	movl	$16, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 70 5 is_stmt 0 discriminator 1
	movl	%eax, -24(%rbp)
	movl	-44(%rbp), %edx
	movl	-24(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -44(%rbp)
	movl	-64(%rbp), %edx
	movl	-44(%rbp), %eax
	xorl	%edx, %eax
	movl	$12, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 70 5 discriminator 2
	movl	%eax, -64(%rbp)
	movl	-68(%rbp), %edx
	movl	-64(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -68(%rbp)
	movl	-24(%rbp), %edx
	movl	-68(%rbp), %eax
	xorl	%edx, %eax
	movl	$8, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 70 5 discriminator 3
	movl	%eax, -24(%rbp)
	movl	-44(%rbp), %edx
	movl	-24(%rbp), %eax
	addl	%edx, %eax
	movl	%eax, -44(%rbp)
	movl	-64(%rbp), %edx
	movl	-44(%rbp), %eax
	xorl	%edx, %eax
	movl	$7, %esi
	movl	%eax, %edi
	call	CRYPTO_rotl_u32
	.loc 1 70 5 discriminator 4
	movl	%eax, -64(%rbp)
	.loc 1 62 32 is_stmt 1 discriminator 3
	addq	$2, -8(%rbp)
.L42:
	.loc 1 62 24 discriminator 1
	cmpq	$19, -8(%rbp)
	jbe	.L43
.LBE2:
	.loc 1 81 3
	leaq	-80(%rbp), %rcx
	movq	-88(%rbp), %rax
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 82 18
	movq	-88(%rbp), %rax
	addq	$16, %rax
	.loc 1 82 3
	leaq	-80(%rbp), %rdx
	leaq	48(%rdx), %rcx
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 84 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE153:
	.size	aws_lc_0_38_0_CRYPTO_hchacha20, .-aws_lc_0_38_0_CRYPTO_hchacha20
	.section	.text.ChaCha20_ctr32,"ax",@progbits
	.type	ChaCha20_ctr32, @function
ChaCha20_ctr32:
.LFB154:
	.loc 1 88 78
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$48, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	movq	%rdx, -24(%rbp)
	movq	%rcx, -32(%rbp)
	movq	%r8, -40(%rbp)
	.loc 1 96 7
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	ChaCha20_ctr32_avx2_capable
	.loc 1 96 6 discriminator 1
	testl	%eax, %eax
	je	.L45
	.loc 1 97 5
	movq	-40(%rbp), %rdi
	movq	-32(%rbp), %rcx
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rsi
	movq	-8(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_ChaCha20_ctr32_avx2@PLT
	.loc 1 98 5
	jmp	.L44
.L45:
	.loc 1 102 7
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	ChaCha20_ctr32_ssse3_4x_capable
	.loc 1 102 6 discriminator 1
	testl	%eax, %eax
	je	.L47
	.loc 1 103 5
	movq	-40(%rbp), %rdi
	movq	-32(%rbp), %rcx
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rsi
	movq	-8(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_ChaCha20_ctr32_ssse3_4x@PLT
	.loc 1 104 5
	jmp	.L44
.L47:
	.loc 1 108 7
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	ChaCha20_ctr32_ssse3_capable
	.loc 1 108 6 discriminator 1
	testl	%eax, %eax
	je	.L48
	.loc 1 109 5
	movq	-40(%rbp), %rdi
	movq	-32(%rbp), %rcx
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rsi
	movq	-8(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_ChaCha20_ctr32_ssse3@PLT
	.loc 1 110 5
	jmp	.L44
.L48:
	.loc 1 113 6
	cmpq	$0, -24(%rbp)
	je	.L44
	.loc 1 114 5
	movq	-40(%rbp), %rdi
	movq	-32(%rbp), %rcx
	movq	-24(%rbp), %rdx
	movq	-16(%rbp), %rsi
	movq	-8(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_ChaCha20_ctr32_nohw@PLT
.L44:
	.loc 1 116 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE154:
	.size	ChaCha20_ctr32, .-ChaCha20_ctr32
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/chacha/chacha.c"
	.align 8
.LC1:
	.string	"!buffers_alias(out, in_len, in, in_len) || in == out"
	.section	.text.aws_lc_0_38_0_CRYPTO_chacha_20,"ax",@progbits
	.globl	aws_lc_0_38_0_CRYPTO_chacha_20
	.type	aws_lc_0_38_0_CRYPTO_chacha_20, @function
aws_lc_0_38_0_CRYPTO_chacha_20:
.LFB155:
	.loc 1 123 41
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$80, %rsp
	movq	%rdi, -40(%rbp)
	movq	%rsi, -48(%rbp)
	movq	%rdx, -56(%rbp)
	movq	%rcx, -64(%rbp)
	movq	%r8, -72(%rbp)
	movl	%r9d, -76(%rbp)
	.loc 1 124 3
	movq	-56(%rbp), %rcx
	movq	-48(%rbp), %rdx
	movq	-56(%rbp), %rsi
	movq	-40(%rbp), %rax
	movq	%rax, %rdi
	call	buffers_alias
	.loc 1 124 3 is_stmt 0 discriminator 1
	testl	%eax, %eax
	je	.L50
	movq	-48(%rbp), %rax
	cmpq	-40(%rbp), %rax
	je	.L50
	.loc 1 124 3 discriminator 2
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$124, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC1(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L50:
	.loc 1 127 20 is_stmt 1
	movl	-76(%rbp), %eax
	movl	%eax, -32(%rbp)
	.loc 1 128 22
	movq	-72(%rbp), %rax
	movq	%rax, %rdi
	call	CRYPTO_load_u32_le
	.loc 1 128 20 discriminator 1
	movl	%eax, -28(%rbp)
	.loc 1 129 47
	movq	-72(%rbp), %rax
	addq	$4, %rax
	.loc 1 129 22
	movq	%rax, %rdi
	call	CRYPTO_load_u32_le
	.loc 1 129 20 discriminator 1
	movl	%eax, -24(%rbp)
	.loc 1 130 47
	movq	-72(%rbp), %rax
	addq	$8, %rax
	.loc 1 130 22
	movq	%rax, %rdi
	call	CRYPTO_load_u32_le
	.loc 1 130 20 discriminator 1
	movl	%eax, -20(%rbp)
	.loc 1 132 19
	movq	-64(%rbp), %rax
	movq	%rax, -16(%rbp)
	.loc 1 150 9
	jmp	.L51
.L53:
.LBB3:
	.loc 1 155 62
	movl	-32(%rbp), %eax
	movl	%eax, %edx
	.loc 1 155 47
	movabsq	$4294967296, %rax
	subq	%rdx, %rax
	.loc 1 155 14
	salq	$6, %rax
	movq	%rax, -8(%rbp)
	.loc 1 156 8
	movq	-8(%rbp), %rax
	cmpq	%rax, -56(%rbp)
	jnb	.L52
	.loc 1 157 12
	movq	-56(%rbp), %rax
	movq	%rax, -8(%rbp)
.L52:
	.loc 1 160 5
	leaq	-32(%rbp), %rdi
	movq	-16(%rbp), %rcx
	movq	-8(%rbp), %rdx
	movq	-48(%rbp), %rsi
	movq	-40(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	ChaCha20_ctr32
	.loc 1 161 8
	movq	-8(%rbp), %rax
	addq	%rax, -48(%rbp)
	.loc 1 162 9
	movq	-8(%rbp), %rax
	addq	%rax, -40(%rbp)
	.loc 1 163 12
	movq	-8(%rbp), %rax
	subq	%rax, -56(%rbp)
	.loc 1 167 22
	movl	$0, -32(%rbp)
.L51:
.LBE3:
	.loc 1 150 17
	cmpq	$0, -56(%rbp)
	jne	.L53
	.loc 1 169 1
	nop
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE155:
	.size	aws_lc_0_38_0_CRYPTO_chacha_20, .-aws_lc_0_38_0_CRYPTO_chacha_20
	.section	.rodata.__PRETTY_FUNCTION__.0,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.0, @object
	.size	__PRETTY_FUNCTION__.0, 31
__PRETTY_FUNCTION__.0:
	.string	"aws_lc_0_38_0_CRYPTO_chacha_20"
	.text
.Letext0:
	.file 5 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 6 "/usr/include/bits/types.h"
	.file 7 "/usr/include/bits/stdint-uintn.h"
	.file 8 "/usr/include/stdint.h"
	.file 9 "/usr/include/assert.h"
	.file 10 "/usr/include/string.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x6c8
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF58
	.byte	0xc
	.long	.LASF59
	.long	.LASF60
	.long	.Ldebug_ranges0+0
	.quad	0
	.long	.Ldebug_line0
	.uleb128 0x2
	.byte	0x8
	.byte	0x5
	.long	.LASF0
	.uleb128 0x3
	.long	.LASF8
	.byte	0x5
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
	.byte	0x6
	.byte	0x26
	.byte	0x17
	.long	0x58
	.uleb128 0x2
	.byte	0x2
	.byte	0x5
	.long	.LASF10
	.uleb128 0x3
	.long	.LASF11
	.byte	0x6
	.byte	0x2a
	.byte	0x16
	.long	0x66
	.uleb128 0x3
	.long	.LASF12
	.byte	0x6
	.byte	0x2d
	.byte	0x1b
	.long	0x3c
	.uleb128 0x5
	.byte	0x8
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF13
	.uleb128 0x6
	.long	0xa1
	.uleb128 0x3
	.long	.LASF14
	.byte	0x7
	.byte	0x18
	.byte	0x13
	.long	0x74
	.uleb128 0x6
	.long	0xad
	.uleb128 0x3
	.long	.LASF15
	.byte	0x7
	.byte	0x1a
	.byte	0x14
	.long	0x87
	.uleb128 0x6
	.long	0xbe
	.uleb128 0x3
	.long	.LASF16
	.byte	0x7
	.byte	0x1b
	.byte	0x14
	.long	0x93
	.uleb128 0x3
	.long	.LASF17
	.byte	0x8
	.byte	0x4f
	.byte	0x1b
	.long	0x3c
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF18
	.uleb128 0x7
	.byte	0x8
	.long	0xf4
	.uleb128 0x8
	.uleb128 0x7
	.byte	0x8
	.long	0xa8
	.uleb128 0x2
	.byte	0x8
	.byte	0x4
	.long	.LASF19
	.uleb128 0x2
	.byte	0x10
	.byte	0x5
	.long	.LASF20
	.uleb128 0x2
	.byte	0x10
	.byte	0x7
	.long	.LASF21
	.uleb128 0x2
	.byte	0x1
	.byte	0x2
	.long	.LASF22
	.uleb128 0x9
	.long	0xbe
	.long	0x127
	.uleb128 0xa
	.long	0x3c
	.byte	0x3
	.byte	0
	.uleb128 0xb
	.long	.LASF61
	.byte	0x3
	.byte	0x28
	.byte	0x11
	.long	0x117
	.uleb128 0x9
	.long	0xca
	.long	0x143
	.uleb128 0xa
	.long	0x3c
	.byte	0x3
	.byte	0
	.uleb128 0x6
	.long	0x133
	.uleb128 0xc
	.long	.LASF32
	.byte	0x1
	.byte	0x1c
	.byte	0x17
	.long	0x143
	.uleb128 0x9
	.byte	0x3
	.quad	sigma_words
	.uleb128 0xd
	.long	.LASF27
	.byte	0x9
	.byte	0x43
	.byte	0xd
	.long	0x17f
	.uleb128 0xe
	.long	0xf5
	.uleb128 0xe
	.long	0xf5
	.uleb128 0xe
	.long	0x66
	.uleb128 0xe
	.long	0xf5
	.byte	0
	.uleb128 0xf
	.long	.LASF23
	.byte	0x4
	.byte	0x5e
	.byte	0x6
	.long	0x1a5
	.uleb128 0xe
	.long	0x1a5
	.uleb128 0xe
	.long	0x1ab
	.uleb128 0xe
	.long	0x30
	.uleb128 0xe
	.long	0x1b1
	.uleb128 0xe
	.long	0x1b1
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xad
	.uleb128 0x7
	.byte	0x8
	.long	0xb9
	.uleb128 0x7
	.byte	0x8
	.long	0xca
	.uleb128 0xf
	.long	.LASF24
	.byte	0x4
	.byte	0x50
	.byte	0x6
	.long	0x1dd
	.uleb128 0xe
	.long	0x1a5
	.uleb128 0xe
	.long	0x1ab
	.uleb128 0xe
	.long	0x30
	.uleb128 0xe
	.long	0x1b1
	.uleb128 0xe
	.long	0x1b1
	.byte	0
	.uleb128 0xf
	.long	.LASF25
	.byte	0x4
	.byte	0x49
	.byte	0x6
	.long	0x203
	.uleb128 0xe
	.long	0x1a5
	.uleb128 0xe
	.long	0x1ab
	.uleb128 0xe
	.long	0x30
	.uleb128 0xe
	.long	0x1b1
	.uleb128 0xe
	.long	0x1b1
	.byte	0
	.uleb128 0xf
	.long	.LASF26
	.byte	0x4
	.byte	0x40
	.byte	0x6
	.long	0x229
	.uleb128 0xe
	.long	0x1a5
	.uleb128 0xe
	.long	0x1ab
	.uleb128 0xe
	.long	0x30
	.uleb128 0xe
	.long	0x1b1
	.uleb128 0xe
	.long	0x1b1
	.byte	0
	.uleb128 0x10
	.long	.LASF28
	.byte	0xa
	.byte	0x2b
	.byte	0xe
	.long	0x9f
	.long	0x249
	.uleb128 0xe
	.long	0x9f
	.uleb128 0xe
	.long	0xee
	.uleb128 0xe
	.long	0x3c
	.byte	0
	.uleb128 0x11
	.long	.LASF36
	.byte	0x1
	.byte	0x79
	.byte	0x6
	.quad	.LFB155
	.quad	.LFE155-.LFB155
	.uleb128 0x1
	.byte	0x9c
	.long	0x317
	.uleb128 0x12
	.string	"out"
	.byte	0x1
	.byte	0x79
	.byte	0x20
	.long	0x1a5
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x12
	.string	"in"
	.byte	0x1
	.byte	0x79
	.byte	0x34
	.long	0x1ab
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x13
	.long	.LASF29
	.byte	0x1
	.byte	0x79
	.byte	0x3f
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x12
	.string	"key"
	.byte	0x1
	.byte	0x7a
	.byte	0x25
	.long	0x1ab
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x13
	.long	.LASF30
	.byte	0x1
	.byte	0x7a
	.byte	0x3c
	.long	0x1ab
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x13
	.long	.LASF31
	.byte	0x1
	.byte	0x7b
	.byte	0x20
	.long	0xbe
	.uleb128 0x3
	.byte	0x91
	.sleb128 -92
	.uleb128 0x14
	.long	.LASF62
	.long	0x327
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.0
	.uleb128 0xc
	.long	.LASF33
	.byte	0x1
	.byte	0x7e
	.byte	0xc
	.long	0x117
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0xc
	.long	.LASF34
	.byte	0x1
	.byte	0x84
	.byte	0x13
	.long	0x1b1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x15
	.quad	.LBB3
	.quad	.LBE3-.LBB3
	.uleb128 0xc
	.long	.LASF35
	.byte	0x1
	.byte	0x9b
	.byte	0xe
	.long	0xcf
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.byte	0
	.uleb128 0x9
	.long	0xa8
	.long	0x327
	.uleb128 0xa
	.long	0x3c
	.byte	0x1e
	.byte	0
	.uleb128 0x6
	.long	0x317
	.uleb128 0x16
	.long	.LASF54
	.byte	0x1
	.byte	0x57
	.byte	0xd
	.quad	.LFB154
	.quad	.LFE154-.LFB154
	.uleb128 0x1
	.byte	0x9c
	.long	0x395
	.uleb128 0x12
	.string	"out"
	.byte	0x1
	.byte	0x57
	.byte	0x25
	.long	0x1a5
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x12
	.string	"in"
	.byte	0x1
	.byte	0x57
	.byte	0x39
	.long	0x1ab
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x13
	.long	.LASF29
	.byte	0x1
	.byte	0x57
	.byte	0x44
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x12
	.string	"key"
	.byte	0x1
	.byte	0x58
	.byte	0x2b
	.long	0x1b1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x13
	.long	.LASF31
	.byte	0x1
	.byte	0x58
	.byte	0x42
	.long	0x1b1
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.byte	0
	.uleb128 0x11
	.long	.LASF37
	.byte	0x1
	.byte	0x2e
	.byte	0x6
	.quad	.LFB153
	.quad	.LFE153-.LFB153
	.uleb128 0x1
	.byte	0x9c
	.long	0x411
	.uleb128 0x12
	.string	"out"
	.byte	0x1
	.byte	0x2e
	.byte	0x1f
	.long	0x1a5
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x12
	.string	"key"
	.byte	0x1
	.byte	0x2e
	.byte	0x36
	.long	0x1ab
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x13
	.long	.LASF30
	.byte	0x1
	.byte	0x2f
	.byte	0x25
	.long	0x1ab
	.uleb128 0x3
	.byte	0x91
	.sleb128 -120
	.uleb128 0x17
	.string	"x"
	.byte	0x1
	.byte	0x30
	.byte	0xc
	.long	0x411
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x15
	.quad	.LBB2
	.quad	.LBE2-.LBB2
	.uleb128 0x17
	.string	"i"
	.byte	0x1
	.byte	0x3e
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.byte	0
	.uleb128 0x9
	.long	0xbe
	.long	0x421
	.uleb128 0xa
	.long	0x3c
	.byte	0xf
	.byte	0
	.uleb128 0x18
	.long	.LASF38
	.byte	0x4
	.byte	0x4d
	.byte	0x14
	.long	0x43
	.quad	.LFB152
	.quad	.LFE152-.LFB152
	.uleb128 0x1
	.byte	0x9c
	.long	0x453
	.uleb128 0x12
	.string	"len"
	.byte	0x4
	.byte	0x4d
	.byte	0x38
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x18
	.long	.LASF39
	.byte	0x4
	.byte	0x44
	.byte	0x14
	.long	0x43
	.quad	.LFB151
	.quad	.LFE151-.LFB151
	.uleb128 0x1
	.byte	0x9c
	.long	0x4a3
	.uleb128 0x12
	.string	"len"
	.byte	0x4
	.byte	0x44
	.byte	0x3b
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0xc
	.long	.LASF40
	.byte	0x4
	.byte	0x45
	.byte	0x7
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0xc
	.long	.LASF41
	.byte	0x4
	.byte	0x46
	.byte	0x7
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x18
	.long	.LASF42
	.byte	0x4
	.byte	0x3d
	.byte	0x14
	.long	0x43
	.quad	.LFB150
	.quad	.LFE150-.LFB150
	.uleb128 0x1
	.byte	0x9c
	.long	0x4d5
	.uleb128 0x12
	.string	"len"
	.byte	0x4
	.byte	0x3d
	.byte	0x37
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x18
	.long	.LASF43
	.byte	0x3
	.byte	0xa3
	.byte	0x14
	.long	0x43
	.quad	.LFB149
	.quad	.LFE149-.LFB149
	.uleb128 0x1
	.byte	0x9c
	.long	0x507
	.uleb128 0xc
	.long	.LASF44
	.byte	0x3
	.byte	0xac
	.byte	0x7
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.uleb128 0x19
	.long	.LASF45
	.byte	0x3
	.byte	0x6b
	.byte	0x14
	.long	0x43
	.quad	.LFB140
	.quad	.LFE140-.LFB140
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x19
	.long	.LASF46
	.byte	0x3
	.byte	0x4e
	.byte	0x14
	.long	0x43
	.quad	.LFB134
	.quad	.LFE134-.LFB134
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x19
	.long	.LASF47
	.byte	0x3
	.byte	0x46
	.byte	0x14
	.long	0x43
	.quad	.LFB132
	.quad	.LFE132-.LFB132
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x1a
	.long	.LASF48
	.byte	0x3
	.byte	0x30
	.byte	0x20
	.long	0x1b1
	.quad	.LFB128
	.quad	.LFE128-.LFB128
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x1b
	.long	.LASF49
	.byte	0x2
	.value	0x472
	.byte	0x18
	.long	0xbe
	.quad	.LFB112
	.quad	.LFE112-.LFB112
	.uleb128 0x1
	.byte	0x9c
	.long	0x5c3
	.uleb128 0x1c
	.long	.LASF50
	.byte	0x2
	.value	0x472
	.byte	0x31
	.long	0xbe
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x1c
	.long	.LASF51
	.byte	0x2
	.value	0x472
	.byte	0x3c
	.long	0x43
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x1d
	.long	.LASF52
	.byte	0x2
	.value	0x3fd
	.byte	0x18
	.long	0xbe
	.quad	.LFB101
	.quad	.LFE101-.LFB101
	.uleb128 0x1
	.byte	0x9c
	.long	0x604
	.uleb128 0x1e
	.string	"in"
	.byte	0x2
	.value	0x3fd
	.byte	0x37
	.long	0xee
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x1f
	.string	"v"
	.byte	0x2
	.value	0x3fe
	.byte	0xc
	.long	0xbe
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.uleb128 0x1d
	.long	.LASF53
	.byte	0x2
	.value	0x3bc
	.byte	0x15
	.long	0x9f
	.quad	.LFB94
	.quad	.LFE94-.LFB94
	.uleb128 0x1
	.byte	0x9c
	.long	0x656
	.uleb128 0x1e
	.string	"dst"
	.byte	0x2
	.value	0x3bc
	.byte	0x2a
	.long	0x9f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x1e
	.string	"src"
	.byte	0x2
	.value	0x3bc
	.byte	0x3b
	.long	0xee
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x1e
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
	.long	.LASF55
	.byte	0x2
	.byte	0xf2
	.byte	0x13
	.long	0x43
	.quad	.LFB63
	.quad	.LFE63-.LFB63
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x12
	.string	"a"
	.byte	0x2
	.byte	0xf2
	.byte	0x30
	.long	0x1ab
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x13
	.long	.LASF56
	.byte	0x2
	.byte	0xf2
	.byte	0x3a
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x12
	.string	"b"
	.byte	0x2
	.byte	0xf3
	.byte	0x30
	.long	0x1ab
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x13
	.long	.LASF57
	.byte	0x2
	.byte	0xf3
	.byte	0x3a
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x17
	.string	"a_u"
	.byte	0x2
	.byte	0xf8
	.byte	0xd
	.long	0xdb
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x17
	.string	"b_u"
	.byte	0x2
	.byte	0xf9
	.byte	0xd
	.long	0xdb
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
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
	.uleb128 0x1
	.byte	0x1
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0xa
	.uleb128 0x21
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2f
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0xb
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
	.uleb128 0xc
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
	.uleb128 0x87
	.uleb128 0x19
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
	.uleb128 0x12
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
	.uleb128 0x13
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
	.uleb128 0x14
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
	.uleb128 0x15
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.byte	0
	.byte	0
	.uleb128 0x16
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
	.uleb128 0x19
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
	.uleb128 0x1a
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
	.uleb128 0x1b
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
	.uleb128 0x1c
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
	.uleb128 0x1d
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
	.uleb128 0x1e
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
	.uleb128 0x1f
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
	.uleb128 0x20
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
	.long	0x10c
	.value	0x2
	.long	.Ldebug_info0
	.byte	0x8
	.byte	0
	.value	0
	.value	0
	.quad	.LFB63
	.quad	.LFE63-.LFB63
	.quad	.LFB94
	.quad	.LFE94-.LFB94
	.quad	.LFB101
	.quad	.LFE101-.LFB101
	.quad	.LFB112
	.quad	.LFE112-.LFB112
	.quad	.LFB128
	.quad	.LFE128-.LFB128
	.quad	.LFB132
	.quad	.LFE132-.LFB132
	.quad	.LFB134
	.quad	.LFE134-.LFB134
	.quad	.LFB140
	.quad	.LFE140-.LFB140
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
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LFB63
	.quad	.LFE63
	.quad	.LFB94
	.quad	.LFE94
	.quad	.LFB101
	.quad	.LFE101
	.quad	.LFB112
	.quad	.LFE112
	.quad	.LFB128
	.quad	.LFE128
	.quad	.LFB132
	.quad	.LFE132
	.quad	.LFB134
	.quad	.LFE134
	.quad	.LFB140
	.quad	.LFE140
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
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF42:
	.string	"ChaCha20_ctr32_avx2_capable"
.LASF16:
	.string	"uint64_t"
.LASF47:
	.string	"CRYPTO_is_SSSE3_capable"
.LASF10:
	.string	"short int"
.LASF8:
	.string	"size_t"
.LASF43:
	.string	"CRYPTO_cpu_perf_is_like_silvermont"
.LASF62:
	.string	"__PRETTY_FUNCTION__"
.LASF54:
	.string	"ChaCha20_ctr32"
.LASF38:
	.string	"ChaCha20_ctr32_ssse3_capable"
.LASF11:
	.string	"__uint32_t"
.LASF20:
	.string	"__int128"
.LASF28:
	.string	"memcpy"
.LASF39:
	.string	"ChaCha20_ctr32_ssse3_4x_capable"
.LASF29:
	.string	"in_len"
.LASF50:
	.string	"value"
.LASF14:
	.string	"uint8_t"
.LASF17:
	.string	"uintptr_t"
.LASF15:
	.string	"uint32_t"
.LASF32:
	.string	"sigma_words"
.LASF21:
	.string	"__int128 unsigned"
.LASF18:
	.string	"long long unsigned int"
.LASF61:
	.string	"aws_lc_0_38_0_OPENSSL_ia32cap_P"
.LASF0:
	.string	"long int"
.LASF51:
	.string	"shift"
.LASF34:
	.string	"key_ptr"
.LASF37:
	.string	"aws_lc_0_38_0_CRYPTO_hchacha20"
.LASF9:
	.string	"__uint8_t"
.LASF55:
	.string	"buffers_alias"
.LASF58:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF3:
	.string	"long double"
.LASF30:
	.string	"nonce"
.LASF4:
	.string	"unsigned char"
.LASF31:
	.string	"counter"
.LASF46:
	.string	"CRYPTO_is_MOVBE_capable"
.LASF7:
	.string	"signed char"
.LASF44:
	.string	"hardware_supports_xsave"
.LASF35:
	.string	"todo"
.LASF6:
	.string	"unsigned int"
.LASF1:
	.string	"long unsigned int"
.LASF56:
	.string	"a_len"
.LASF5:
	.string	"short unsigned int"
.LASF25:
	.string	"aws_lc_0_38_0_ChaCha20_ctr32_ssse3_4x"
.LASF13:
	.string	"char"
.LASF26:
	.string	"aws_lc_0_38_0_ChaCha20_ctr32_avx2"
.LASF59:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/chacha/chacha.c"
.LASF22:
	.string	"_Bool"
.LASF45:
	.string	"CRYPTO_is_AVX2_capable"
.LASF12:
	.string	"__uint64_t"
.LASF24:
	.string	"aws_lc_0_38_0_ChaCha20_ctr32_ssse3"
.LASF33:
	.string	"counter_nonce"
.LASF40:
	.string	"capable"
.LASF19:
	.string	"double"
.LASF36:
	.string	"aws_lc_0_38_0_CRYPTO_chacha_20"
.LASF2:
	.string	"long long int"
.LASF49:
	.string	"CRYPTO_rotl_u32"
.LASF23:
	.string	"aws_lc_0_38_0_ChaCha20_ctr32_nohw"
.LASF60:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF52:
	.string	"CRYPTO_load_u32_le"
.LASF57:
	.string	"b_len"
.LASF27:
	.string	"__assert_fail"
.LASF41:
	.string	"faster"
.LASF53:
	.string	"OPENSSL_memcpy"
.LASF48:
	.string	"OPENSSL_ia32cap_get"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
