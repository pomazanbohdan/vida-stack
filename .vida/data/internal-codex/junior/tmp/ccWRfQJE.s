	.file	"e_aesgcmsiv.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aesgcmsiv.c"
	.section	.text.OPENSSL_memcpy,"ax",@progbits
	.type	OPENSSL_memcpy, @function
OPENSSL_memcpy:
.LFB94:
	.file 2 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/../fipsmodule/cipher/../../internal.h"
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
	.section	.text.CRYPTO_store_u32_le,"ax",@progbits
	.type	CRYPTO_store_u32_le, @function
CRYPTO_store_u32_le:
.LFB102:
	.loc 2 1031 63
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movl	%esi, -12(%rbp)
	.loc 2 1035 3
	leaq	-12(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$4, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 2 1037 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE102:
	.size	CRYPTO_store_u32_le, .-CRYPTO_store_u32_le
	.section	.text.CRYPTO_store_u64_le,"ax",@progbits
	.type	CRYPTO_store_u64_le, @function
CRYPTO_store_u64_le:
.LFB106:
	.loc 2 1068 63
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	%rsi, -16(%rbp)
	.loc 2 1072 3
	leaq	-16(%rbp), %rcx
	movq	-8(%rbp), %rax
	movl	$8, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 2 1074 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE106:
	.size	CRYPTO_store_u64_le, .-CRYPTO_store_u64_le
	.section	.text.OPENSSL_ia32cap_get,"ax",@progbits
	.type	OPENSSL_ia32cap_get, @function
OPENSSL_ia32cap_get:
.LFB128:
	.file 3 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/../fipsmodule/cipher/../modes/../cpucap/internal.h"
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
	.section	.text.CRYPTO_is_AESNI_capable,"ax",@progbits
	.type	CRYPTO_is_AESNI_capable, @function
CRYPTO_is_AESNI_capable:
.LFB135:
	.loc 3 82 50
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 3 83 11
	call	OPENSSL_ia32cap_get
	.loc 3 83 32 discriminator 1
	addq	$4, %rax
	movl	(%rax), %eax
	.loc 3 83 36 discriminator 1
	andl	$33554432, %eax
	.loc 3 83 49 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 3 84 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE135:
	.size	CRYPTO_is_AESNI_capable, .-CRYPTO_is_AESNI_capable
	.section	.text.CRYPTO_is_AVX_capable,"ax",@progbits
	.type	CRYPTO_is_AVX_capable, @function
CRYPTO_is_AVX_capable:
.LFB136:
	.loc 3 89 48
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 3 90 11
	call	OPENSSL_ia32cap_get
	.loc 3 90 32 discriminator 1
	addq	$4, %rax
	movl	(%rax), %eax
	.loc 3 90 36 discriminator 1
	andl	$268435456, %eax
	.loc 3 90 49 discriminator 1
	testl	%eax, %eax
	setne	%al
	movzbl	%al, %eax
	.loc 3 91 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE136:
	.size	CRYPTO_is_AVX_capable, .-CRYPTO_is_AVX_capable
	.section	.text.asm_ctx_from_ctx,"ax",@progbits
	.type	asm_ctx_from_ctx, @function
asm_ctx_from_ctx:
.LFB152:
	.loc 1 54 30
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -24(%rbp)
	.loc 1 57 47
	movq	-24(%rbp), %rax
	addq	$8, %rax
	.loc 1 57 19
	andl	$8, %eax
	movq	%rax, -8(%rbp)
	.loc 1 58 9
	movq	-24(%rbp), %rax
	movzbl	576(%rax), %eax
	movzbl	%al, %eax
	.loc 1 58 5
	cmpq	%rax, -8(%rbp)
	je	.L18
	.loc 1 59 12
	movl	$0, %eax
	jmp	.L19
.L18:
	.loc 1 61 10
	movq	-24(%rbp), %rdx
	movq	-8(%rbp), %rax
	addq	%rdx, %rax
	addq	$8, %rax
.L19:
	.loc 1 62 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE152:
	.size	asm_ctx_from_ctx, .-asm_ctx_from_ctx
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aesgcmsiv.c"
	.align 8
.LC1:
	.string	"(((uintptr_t)gcm_siv_ctx) & 15) == 0"
	.section	.text.aead_aes_gcm_siv_asm_init,"ax",@progbits
	.type	aead_aes_gcm_siv_asm_init, @function
aead_aes_gcm_siv_asm_init:
.LFB153:
	.loc 1 75 70
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
	movq	%rcx, -48(%rbp)
	.loc 1 76 16
	movq	-40(%rbp), %rax
	salq	$3, %rax
	movq	%rax, -8(%rbp)
	.loc 1 78 6
	cmpq	$128, -8(%rbp)
	je	.L21
	.loc 1 78 23 discriminator 1
	cmpq	$256, -8(%rbp)
	je	.L21
	.loc 1 79 5
	movl	$79, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$102, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 80 12
	movl	$0, %eax
	jmp	.L22
.L21:
	.loc 1 83 6
	cmpq	$0, -48(%rbp)
	jne	.L23
	.loc 1 84 13
	movq	$16, -48(%rbp)
.L23:
	.loc 1 87 6
	cmpq	$16, -48(%rbp)
	je	.L24
	.loc 1 88 5
	movl	$88, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$116, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 89 12
	movl	$0, %eax
	jmp	.L22
.L24:
	.loc 1 92 35
	movq	-24(%rbp), %rax
	addq	$8, %rax
	.loc 1 92 48
	andl	$8, %eax
	movl	%eax, %edx
	.loc 1 92 21
	movq	-24(%rbp), %rax
	movb	%dl, 576(%rax)
	.loc 1 93 50
	movq	-24(%rbp), %rax
	movq	%rax, %rdi
	call	asm_ctx_from_ctx
	movq	%rax, -16(%rbp)
	.loc 1 94 5
	cmpq	$0, -16(%rbp)
	jne	.L25
	.loc 1 95 5
	movl	$95, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$107, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 96 12
	movl	$0, %eax
	jmp	.L22
.L25:
	.loc 1 98 3
	movq	-16(%rbp), %rax
	andl	$15, %eax
	testq	%rax, %rax
	je	.L26
	.loc 1 98 3 is_stmt 0 discriminator 1
	leaq	__PRETTY_FUNCTION__.0(%rip), %rax
	movq	%rax, %rcx
	movl	$98, %edx
	leaq	.LC0(%rip), %rax
	movq	%rax, %rsi
	leaq	.LC1(%rip), %rax
	movq	%rax, %rdi
	call	__assert_fail@PLT
.L26:
	.loc 1 100 6 is_stmt 1
	cmpq	$128, -8(%rbp)
	jne	.L27
	.loc 1 101 5
	movq	-16(%rbp), %rdx
	movq	-32(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes128gcmsiv_aes_ks@PLT
	.loc 1 102 29
	movq	-16(%rbp), %rax
	movl	$1, 240(%rax)
	jmp	.L28
.L27:
	.loc 1 104 5
	movq	-16(%rbp), %rdx
	movq	-32(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes256gcmsiv_aes_ks@PLT
	.loc 1 105 29
	movq	-16(%rbp), %rax
	movl	$0, 240(%rax)
.L28:
	.loc 1 108 16
	movq	-48(%rbp), %rax
	movl	%eax, %edx
	movq	-24(%rbp), %rax
	movb	%dl, 577(%rax)
	.loc 1 110 10
	movl	$1, %eax
.L22:
	.loc 1 111 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE153:
	.size	aead_aes_gcm_siv_asm_init, .-aead_aes_gcm_siv_asm_init
	.section	.text.aead_aes_gcm_siv_asm_cleanup,"ax",@progbits
	.type	aead_aes_gcm_siv_asm_cleanup, @function
aead_aes_gcm_siv_asm_cleanup:
.LFB154:
	.loc 1 113 61
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	.loc 1 113 62
	nop
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE154:
	.size	aead_aes_gcm_siv_asm_cleanup, .-aead_aes_gcm_siv_asm_cleanup
	.section	.text.gcm_siv_asm_polyval,"ax",@progbits
	.type	gcm_siv_asm_polyval, @function
gcm_siv_asm_polyval:
.LFB155:
	.loc 1 235 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$240, %rsp
	movq	%rdi, -200(%rbp)
	movq	%rsi, -208(%rbp)
	movq	%rdx, -216(%rbp)
	movq	%rcx, -224(%rbp)
	movq	%r8, -232(%rbp)
	movq	%r9, -240(%rbp)
	.loc 1 236 3
	movq	-200(%rbp), %rax
	movl	$16, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 237 16
	movq	-232(%rbp), %rax
	shrq	$4, %rax
	movq	%rax, -24(%rbp)
	.loc 1 238 16
	movq	-216(%rbp), %rax
	shrq	$4, %rax
	movq	%rax, -32(%rbp)
	.loc 1 239 7
	movl	$0, -4(%rbp)
	.loc 1 242 6
	cmpq	$8, -24(%rbp)
	ja	.L31
	.loc 1 242 21 discriminator 1
	cmpq	$8, -32(%rbp)
	jbe	.L32
.L31:
	.loc 1 243 17
	movl	$1, -4(%rbp)
	.loc 1 244 5
	movq	-240(%rbp), %rdx
	leaq	-160(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_htable_init@PLT
.L32:
	.loc 1 247 6
	cmpl	$0, -4(%rbp)
	je	.L33
	.loc 1 248 5
	movq	-232(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rdi
	movq	-200(%rbp), %rdx
	movq	-224(%rbp), %rsi
	leaq	-160(%rbp), %rax
	movq	%rdx, %rcx
	movq	%rdi, %rdx
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_htable_polyval@PLT
	jmp	.L34
.L33:
	.loc 1 250 5
	movq	-24(%rbp), %rcx
	movq	-224(%rbp), %rdx
	movq	-240(%rbp), %rsi
	movq	-200(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_polyval_horner@PLT
.L34:
	.loc 1 254 14
	movq	-232(%rbp), %rax
	andl	$15, %eax
	.loc 1 254 6
	testq	%rax, %rax
	je	.L35
	.loc 1 255 5
	leaq	-176(%rbp), %rax
	movl	$16, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 256 5
	movq	-232(%rbp), %rax
	andl	$15, %eax
	movq	%rax, %rdx
	.loc 1 256 40
	movq	-232(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rcx
	.loc 1 256 29
	movq	-224(%rbp), %rax
	addq	%rax, %rcx
	.loc 1 256 5
	leaq	-176(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 257 5
	leaq	-176(%rbp), %rdx
	movq	-240(%rbp), %rsi
	movq	-200(%rbp), %rax
	movl	$1, %ecx
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_polyval_horner@PLT
.L35:
	.loc 1 260 6
	cmpl	$0, -4(%rbp)
	je	.L36
	.loc 1 261 5
	movq	-216(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rdi
	movq	-200(%rbp), %rdx
	movq	-208(%rbp), %rsi
	leaq	-160(%rbp), %rax
	movq	%rdx, %rcx
	movq	%rdi, %rdx
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_htable_polyval@PLT
	jmp	.L37
.L36:
	.loc 1 263 5
	movq	-32(%rbp), %rcx
	movq	-208(%rbp), %rdx
	movq	-240(%rbp), %rsi
	movq	-200(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_polyval_horner@PLT
.L37:
	.loc 1 266 14
	movq	-216(%rbp), %rax
	andl	$15, %eax
	.loc 1 266 6
	testq	%rax, %rax
	je	.L38
	.loc 1 267 5
	leaq	-176(%rbp), %rax
	movl	$16, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 268 5
	movq	-216(%rbp), %rax
	andl	$15, %eax
	movq	%rax, %rdx
	.loc 1 268 40
	movq	-216(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rcx
	.loc 1 268 29
	movq	-208(%rbp), %rax
	addq	%rax, %rcx
	.loc 1 268 5
	leaq	-176(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 269 5
	leaq	-176(%rbp), %rdx
	movq	-240(%rbp), %rsi
	movq	-200(%rbp), %rax
	movl	$1, %ecx
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_polyval_horner@PLT
.L38:
	.loc 1 273 3
	movq	-232(%rbp), %rax
	leaq	0(,%rax,8), %rdx
	leaq	-192(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	CRYPTO_store_u64_le
	.loc 1 274 3
	movq	-216(%rbp), %rax
	leaq	0(,%rax,8), %rdx
	leaq	-192(%rbp), %rax
	addq	$8, %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	CRYPTO_store_u64_le
	.loc 1 275 3
	leaq	-192(%rbp), %rdx
	movq	-240(%rbp), %rsi
	movq	-200(%rbp), %rax
	movl	$1, %ecx
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_polyval_horner@PLT
.LBB2:
	.loc 1 277 15
	movq	$0, -16(%rbp)
	.loc 1 277 3
	jmp	.L39
.L40:
	.loc 1 278 12
	movq	-200(%rbp), %rdx
	movq	-16(%rbp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %esi
	.loc 1 278 24
	movq	16(%rbp), %rdx
	movq	-16(%rbp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %ecx
	.loc 1 278 12
	movq	-200(%rbp), %rdx
	movq	-16(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 278 16
	xorl	%ecx, %esi
	movl	%esi, %edx
	movb	%dl, (%rax)
	.loc 1 277 31 discriminator 3
	addq	$1, -16(%rbp)
.L39:
	.loc 1 277 24 discriminator 1
	cmpq	$11, -16(%rbp)
	jbe	.L40
.LBE2:
	.loc 1 281 10
	movq	-200(%rbp), %rax
	addq	$15, %rax
	movzbl	(%rax), %edx
	movq	-200(%rbp), %rax
	addq	$15, %rax
	.loc 1 281 15
	andl	$127, %edx
	movb	%dl, (%rax)
	.loc 1 282 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE155:
	.size	gcm_siv_asm_polyval, .-gcm_siv_asm_polyval
	.section	.text.aead_aes_gcm_siv_asm_crypt_last_block,"ax",@progbits
	.type	aead_aes_gcm_siv_asm_crypt_last_block, @function
aead_aes_gcm_siv_asm_crypt_last_block:
.LFB156:
	.loc 1 291 62
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$112, %rsp
	movl	%edi, -68(%rbp)
	movq	%rsi, -80(%rbp)
	movq	%rdx, -88(%rbp)
	movq	%rcx, -96(%rbp)
	movq	%r8, -104(%rbp)
	movq	%r9, -112(%rbp)
	.loc 1 293 3
	movq	-104(%rbp), %rcx
	leaq	-64(%rbp), %rax
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 294 10
	movzbl	-49(%rbp), %eax
	.loc 1 294 15
	orl	$-128, %eax
	movb	%al, -49(%rbp)
	.loc 1 295 32
	leaq	-64(%rbp), %rax
	movq	%rax, %rdi
	call	CRYPTO_load_u32_le
	.loc 1 295 69 discriminator 1
	movq	-96(%rbp), %rdx
	shrq	$4, %rdx
	.loc 1 295 3 discriminator 1
	addl	%eax, %edx
	leaq	-64(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	CRYPTO_store_u32_le
	.loc 1 297 6
	cmpl	$0, -68(%rbp)
	je	.L42
	.loc 1 298 5
	movq	-112(%rbp), %rdx
	leaq	-64(%rbp), %rcx
	leaq	-64(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes128gcmsiv_ecb_enc_block@PLT
	jmp	.L43
.L42:
	.loc 1 300 5
	movq	-112(%rbp), %rdx
	leaq	-64(%rbp), %rcx
	leaq	-64(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes256gcmsiv_ecb_enc_block@PLT
.L43:
	.loc 1 303 16
	movq	-96(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, -16(%rbp)
	.loc 1 304 16
	movq	-96(%rbp), %rax
	andl	$15, %eax
	movq	%rax, -24(%rbp)
	.loc 1 305 12
	movq	-80(%rbp), %rdx
	movq	-16(%rbp), %rax
	addq	%rdx, %rax
	movq	%rax, -32(%rbp)
	.loc 1 306 18
	movq	-88(%rbp), %rdx
	movq	-16(%rbp), %rax
	addq	%rdx, %rax
	movq	%rax, -40(%rbp)
.LBB3:
	.loc 1 307 15
	movq	$0, -8(%rbp)
	.loc 1 307 3
	jmp	.L44
.L45:
	.loc 1 308 38
	movq	-40(%rbp), %rdx
	movq	-8(%rbp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %esi
	.loc 1 308 51
	leaq	-64(%rbp), %rdx
	movq	-8(%rbp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %ecx
	.loc 1 308 19
	movq	-32(%rbp), %rdx
	movq	-8(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 308 23
	xorl	%ecx, %esi
	movl	%esi, %edx
	movb	%dl, (%rax)
	.loc 1 307 43 discriminator 3
	addq	$1, -8(%rbp)
.L44:
	.loc 1 307 24 discriminator 1
	movq	-8(%rbp), %rax
	cmpq	-24(%rbp), %rax
	jb	.L45
.LBE3:
	.loc 1 310 1
	nop
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE156:
	.size	aead_aes_gcm_siv_asm_crypt_last_block, .-aead_aes_gcm_siv_asm_crypt_last_block
	.section	.text.aead_aes_gcm_siv_kdf,"ax",@progbits
	.type	aead_aes_gcm_siv_kdf, @function
aead_aes_gcm_siv_kdf:
.LFB157:
	.loc 1 317 30
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$160, %rsp
	movl	%edi, -116(%rbp)
	movq	%rsi, -128(%rbp)
	movq	%rdx, -136(%rbp)
	movq	%rcx, -144(%rbp)
	movq	%r8, -152(%rbp)
	.loc 1 319 3
	movq	-152(%rbp), %rcx
	leaq	-16(%rbp), %rax
	movl	$12, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 322 6
	cmpl	$0, -116(%rbp)
	je	.L47
	.loc 1 323 5
	movq	-128(%rbp), %rdx
	leaq	-112(%rbp), %rcx
	leaq	-16(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes128gcmsiv_kdf@PLT
	.loc 1 324 41
	movq	-80(%rbp), %rdx
	.loc 1 324 27
	movq	-144(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 325 23
	movq	-144(%rbp), %rax
	leaq	8(%rax), %rdx
	.loc 1 325 41
	movq	-64(%rbp), %rax
	.loc 1 325 27
	movq	%rax, (%rdx)
	jmp	.L48
.L47:
	.loc 1 327 5
	movq	-128(%rbp), %rdx
	leaq	-112(%rbp), %rcx
	leaq	-16(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes256gcmsiv_kdf@PLT
	.loc 1 328 41
	movq	-80(%rbp), %rdx
	.loc 1 328 27
	movq	-144(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 329 23
	movq	-144(%rbp), %rax
	leaq	8(%rax), %rdx
	.loc 1 329 41
	movq	-64(%rbp), %rax
	.loc 1 329 27
	movq	%rax, (%rdx)
	.loc 1 330 23
	movq	-144(%rbp), %rax
	leaq	16(%rax), %rdx
	.loc 1 330 41
	movq	-48(%rbp), %rax
	.loc 1 330 27
	movq	%rax, (%rdx)
	.loc 1 331 23
	movq	-144(%rbp), %rax
	leaq	24(%rax), %rdx
	.loc 1 331 41
	movq	-32(%rbp), %rax
	.loc 1 331 27
	movq	%rax, (%rdx)
.L48:
	.loc 1 334 40
	movq	-112(%rbp), %rdx
	.loc 1 334 26
	movq	-136(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 335 22
	movq	-136(%rbp), %rax
	leaq	8(%rax), %rdx
	.loc 1 335 40
	movq	-96(%rbp), %rax
	.loc 1 335 26
	movq	%rax, (%rdx)
	.loc 1 336 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE157:
	.size	aead_aes_gcm_siv_kdf, .-aead_aes_gcm_siv_kdf
	.section	.text.aead_aes_gcm_siv_asm_seal_scatter,"ax",@progbits
	.type	aead_aes_gcm_siv_asm_seal_scatter, @function
aead_aes_gcm_siv_asm_seal_scatter:
.LFB158:
	.loc 1 342 60
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$400, %rsp
	movq	%rdi, -360(%rbp)
	movq	%rsi, -368(%rbp)
	movq	%rdx, -376(%rbp)
	movq	%rcx, -384(%rbp)
	movq	%r8, -392(%rbp)
	movq	%r9, -400(%rbp)
	.loc 1 343 56
	movq	-360(%rbp), %rax
	movq	%rax, %rdi
	call	asm_ctx_from_ctx
	movq	%rax, -8(%rbp)
	.loc 1 344 5
	cmpq	$0, -8(%rbp)
	jne	.L50
	.loc 1 345 5
	movl	$345, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$142, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 346 12
	movl	$0, %eax
	jmp	.L61
.L50:
	.loc 1 348 18
	movq	32(%rbp), %rax
	movq	%rax, -16(%rbp)
	.loc 1 349 18
	movq	64(%rbp), %rax
	movq	%rax, -24(%rbp)
	.loc 1 351 6
	movabsq	$68719476736, %rax
	cmpq	-16(%rbp), %rax
	jb	.L52
	.loc 1 351 39 discriminator 1
	movabsq	$2305843009213693951, %rax
	cmpq	-24(%rbp), %rax
	jnb	.L53
.L52:
	.loc 1 352 5
	movl	$352, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$117, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 353 12
	movl	$0, %eax
	jmp	.L61
.L53:
	.loc 1 356 6
	cmpq	$15, -392(%rbp)
	ja	.L54
	.loc 1 357 5
	movl	$357, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$103, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 358 12
	movl	$0, %eax
	jmp	.L61
.L54:
	.loc 1 361 6
	cmpq	$12, 16(%rbp)
	je	.L55
	.loc 1 362 5
	movl	$362, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$121, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 363 12
	movl	$0, %eax
	jmp	.L61
.L55:
	.loc 1 368 35
	movq	-8(%rbp), %rax
	movl	240(%rax), %eax
	.loc 1 368 3
	movq	-400(%rbp), %rdi
	leaq	-80(%rbp), %rcx
	leaq	-48(%rbp), %rdx
	movq	-8(%rbp), %rsi
	movq	%rdi, %r8
	movl	%eax, %edi
	call	aead_aes_gcm_siv_kdf
	.loc 1 371 23
	movq	$0, -96(%rbp)
	movq	$0, -88(%rbp)
	.loc 1 372 3
	leaq	-48(%rbp), %r8
	movq	64(%rbp), %rdi
	movq	56(%rbp), %rcx
	movq	32(%rbp), %rdx
	movq	24(%rbp), %rsi
	leaq	-96(%rbp), %rax
	subq	$8, %rsp
	pushq	-400(%rbp)
	movq	%r8, %r9
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	gcm_siv_asm_polyval
	addq	$16, %rsp
	.loc 1 377 18
	movq	-8(%rbp), %rax
	movl	240(%rax), %eax
	.loc 1 377 6
	testl	%eax, %eax
	je	.L56
	.loc 1 378 5
	leaq	-80(%rbp), %rcx
	leaq	-352(%rbp), %rdx
	leaq	-96(%rbp), %rsi
	leaq	-96(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes128gcmsiv_aes_ks_enc_x1@PLT
	.loc 1 381 8
	cmpq	$127, 32(%rbp)
	ja	.L57
	.loc 1 382 7
	movq	32(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rdi
	leaq	-352(%rbp), %rcx
	leaq	-96(%rbp), %rdx
	movq	-368(%rbp), %rsi
	movq	24(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes128gcmsiv_enc_msg_x4@PLT
	jmp	.L58
.L57:
	.loc 1 384 7
	movq	32(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rdi
	leaq	-352(%rbp), %rcx
	leaq	-96(%rbp), %rdx
	movq	-368(%rbp), %rsi
	movq	24(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes128gcmsiv_enc_msg_x8@PLT
	jmp	.L58
.L56:
	.loc 1 387 5
	leaq	-80(%rbp), %rcx
	leaq	-352(%rbp), %rdx
	leaq	-96(%rbp), %rsi
	leaq	-96(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes256gcmsiv_aes_ks_enc_x1@PLT
	.loc 1 390 8
	cmpq	$127, 32(%rbp)
	ja	.L59
	.loc 1 391 7
	movq	32(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rdi
	leaq	-352(%rbp), %rcx
	leaq	-96(%rbp), %rdx
	movq	-368(%rbp), %rsi
	movq	24(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes256gcmsiv_enc_msg_x4@PLT
	jmp	.L58
.L59:
	.loc 1 393 7
	movq	32(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rdi
	leaq	-352(%rbp), %rcx
	leaq	-96(%rbp), %rdx
	movq	-368(%rbp), %rsi
	movq	24(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes256gcmsiv_enc_msg_x8@PLT
.L58:
	.loc 1 397 14
	movq	32(%rbp), %rax
	andl	$15, %eax
	.loc 1 397 6
	testq	%rax, %rax
	je	.L60
	.loc 1 398 54
	movq	-8(%rbp), %rax
	movl	240(%rax), %eax
	.loc 1 398 5
	leaq	-352(%rbp), %r8
	leaq	-96(%rbp), %rdi
	movq	32(%rbp), %rcx
	movq	24(%rbp), %rdx
	movq	-368(%rbp), %rsi
	movq	%r8, %r9
	movq	%rdi, %r8
	movl	%eax, %edi
	call	aead_aes_gcm_siv_asm_crypt_last_block
.L60:
	.loc 1 402 3
	leaq	-96(%rbp), %rcx
	movq	-376(%rbp), %rax
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 403 16
	movq	-384(%rbp), %rax
	movq	$16, (%rax)
	.loc 1 405 10
	movl	$1, %eax
.L61:
	.loc 1 406 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE158:
	.size	aead_aes_gcm_siv_asm_seal_scatter, .-aead_aes_gcm_siv_asm_seal_scatter
	.section	.text.aead_aes_gcm_siv_asm_open_gather,"ax",@progbits
	.type	aead_aes_gcm_siv_asm_open_gather, @function
aead_aes_gcm_siv_asm_open_gather:
.LFB159:
	.loc 1 411 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	pushq	%rbx
	subq	$664, %rsp
	.cfi_offset 3, -24
	movq	%rdi, 40(%rsp)
	movq	%rsi, 32(%rsp)
	movq	%rdx, 24(%rsp)
	movq	%rcx, 16(%rsp)
	movq	%r8, 8(%rsp)
	movq	%r9, (%rsp)
	.loc 1 412 18
	movq	40(%rbp), %rax
	movq	%rax, 640(%rsp)
	.loc 1 413 6
	movabsq	$2305843009213693951, %rax
	cmpq	640(%rsp), %rax
	jnb	.L63
	.loc 1 414 5
	movl	$414, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$117, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 415 12
	movl	$0, %eax
	jmp	.L80
.L63:
	.loc 1 418 18
	movq	(%rsp), %rax
	movq	%rax, 632(%rsp)
	.loc 1 419 6
	movabsq	$68719476736, %rax
	cmpq	632(%rsp), %rax
	jb	.L65
	.loc 1 419 37 discriminator 1
	cmpq	$16, 24(%rbp)
	je	.L66
.L65:
	.loc 1 421 5
	movl	$421, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 422 12
	movl	$0, %eax
	jmp	.L80
.L66:
	.loc 1 425 6
	cmpq	$12, 16(%rsp)
	je	.L67
	.loc 1 426 5
	movl	$426, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$121, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 427 12
	movl	$0, %eax
	jmp	.L80
.L67:
	.loc 1 430 56
	movq	40(%rsp), %rax
	movq	%rax, %rdi
	call	asm_ctx_from_ctx
	movq	%rax, 624(%rsp)
	.loc 1 431 5
	cmpq	$0, 624(%rsp)
	jne	.L68
	.loc 1 432 5
	movl	$432, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$142, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 433 12
	movl	$0, %eax
	jmp	.L80
.L68:
	.loc 1 438 35
	movq	624(%rsp), %rax
	movl	240(%rax), %eax
	.loc 1 438 3
	movq	24(%rsp), %rdi
	leaq	560(%rsp), %rcx
	leaq	592(%rsp), %rdx
	movq	624(%rsp), %rsi
	movq	%rdi, %r8
	movl	%eax, %edi
	call	aead_aes_gcm_siv_kdf
	.loc 1 442 18
	movq	624(%rsp), %rax
	movl	240(%rax), %eax
	.loc 1 442 6
	testl	%eax, %eax
	je	.L69
	.loc 1 443 5
	leaq	304(%rsp), %rdx
	leaq	560(%rsp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes128gcmsiv_aes_ks@PLT
	jmp	.L70
.L69:
	.loc 1 445 5
	leaq	304(%rsp), %rdx
	leaq	560(%rsp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes256gcmsiv_aes_ks@PLT
.L70:
	.loc 1 449 23
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%ymm0, 176(%rsp)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%ymm0, 208(%rsp)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%ymm0, 240(%rsp)
	vpxor	%xmm0, %xmm0, %xmm0
	vmovdqu	%ymm0, 272(%rsp)
	.loc 1 451 3
	leaq	176(%rsp), %rax
	movl	$16, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 452 16
	movq	40(%rbp), %rax
	shrq	$4, %rax
	movq	%rax, 616(%rsp)
	.loc 1 453 3
	movq	616(%rsp), %rcx
	movq	32(%rbp), %rdx
	leaq	592(%rsp), %rsi
	leaq	176(%rsp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_polyval_horner@PLT
	.loc 1 457 14
	movq	40(%rbp), %rax
	andl	$15, %eax
	.loc 1 457 6
	testq	%rax, %rax
	je	.L71
	.loc 1 458 5
	leaq	160(%rsp), %rax
	movl	$16, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 459 5
	movq	40(%rbp), %rax
	andl	$15, %eax
	movq	%rax, %rdx
	.loc 1 459 40
	movq	40(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rcx
	.loc 1 459 29
	movq	32(%rbp), %rax
	addq	%rax, %rcx
	.loc 1 459 5
	leaq	160(%rsp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 460 5
	leaq	160(%rsp), %rdx
	leaq	592(%rsp), %rsi
	leaq	176(%rsp), %rax
	movl	$1, %ecx
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_polyval_horner@PLT
.L71:
	.loc 1 465 3
	leaq	592(%rsp), %rdx
	leaq	64(%rsp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_htable6_init@PLT
	.loc 1 469 3
	leaq	176(%rsp), %rax
	addq	$16, %rax
	movq	16(%rbp), %rdx
	movq	(%rdx), %rcx
	movq	8(%rdx), %rbx
	movq	%rcx, (%rax)
	movq	%rbx, 8(%rax)
	.loc 1 470 18
	movq	624(%rsp), %rax
	movl	240(%rax), %eax
	.loc 1 470 6
	testl	%eax, %eax
	je	.L72
	.loc 1 471 5
	movq	(%rsp), %r8
	leaq	304(%rsp), %rdi
	leaq	64(%rsp), %rcx
	leaq	176(%rsp), %rdx
	movq	32(%rsp), %rsi
	movq	8(%rsp), %rax
	movq	%r8, %r9
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes128gcmsiv_dec@PLT
	jmp	.L73
.L72:
	.loc 1 473 5
	movq	(%rsp), %r8
	leaq	304(%rsp), %rdi
	leaq	64(%rsp), %rcx
	leaq	176(%rsp), %rdx
	movq	32(%rsp), %rsi
	movq	8(%rsp), %rax
	movq	%r8, %r9
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes256gcmsiv_dec@PLT
.L73:
	.loc 1 476 14
	movq	(%rsp), %rax
	andl	$15, %eax
	.loc 1 476 6
	testq	%rax, %rax
	je	.L74
	.loc 1 477 54
	movq	624(%rsp), %rax
	movl	240(%rax), %eax
	.loc 1 477 5
	leaq	304(%rsp), %r8
	movq	16(%rbp), %rdi
	movq	(%rsp), %rcx
	movq	8(%rsp), %rdx
	movq	32(%rsp), %rsi
	movq	%r8, %r9
	movq	%rdi, %r8
	movl	%eax, %edi
	call	aead_aes_gcm_siv_asm_crypt_last_block
	.loc 1 479 5
	leaq	160(%rsp), %rax
	movl	$16, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 480 5
	movq	(%rsp), %rax
	andl	$15, %eax
	movq	%rax, %rdx
	.loc 1 480 43
	movq	(%rsp), %rax
	andq	$-16, %rax
	movq	%rax, %rcx
	.loc 1 480 33
	movq	32(%rsp), %rax
	addq	%rax, %rcx
	.loc 1 480 5
	leaq	160(%rsp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 481 5
	leaq	160(%rsp), %rdx
	leaq	592(%rsp), %rsi
	leaq	176(%rsp), %rax
	movl	$1, %ecx
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_polyval_horner@PLT
.L74:
	.loc 1 486 3
	movq	40(%rbp), %rax
	leaq	0(,%rax,8), %rdx
	leaq	48(%rsp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	CRYPTO_store_u64_le
	.loc 1 487 3
	movq	(%rsp), %rax
	leaq	0(,%rax,8), %rdx
	leaq	48(%rsp), %rax
	addq	$8, %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	CRYPTO_store_u64_le
	.loc 1 488 3
	leaq	48(%rsp), %rdx
	leaq	592(%rsp), %rsi
	leaq	176(%rsp), %rax
	movl	$1, %ecx
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aesgcmsiv_polyval_horner@PLT
.LBB4:
	.loc 1 491 15
	movq	$0, 648(%rsp)
	.loc 1 491 3
	jmp	.L75
.L76:
	.loc 1 492 19
	leaq	176(%rsp), %rdx
	movq	648(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %ecx
	.loc 1 492 31
	movq	24(%rsp), %rdx
	movq	648(%rsp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %eax
	.loc 1 492 23
	xorl	%eax, %ecx
	movl	%ecx, %edx
	leaq	176(%rsp), %rcx
	movq	648(%rsp), %rax
	addq	%rcx, %rax
	movb	%dl, (%rax)
	.loc 1 491 31 discriminator 3
	addq	$1, 648(%rsp)
.L75:
	.loc 1 491 24 discriminator 1
	cmpq	$11, 648(%rsp)
	jbe	.L76
.LBE4:
	.loc 1 495 17
	movzbl	191(%rsp), %eax
	.loc 1 495 22
	andl	$127, %eax
	movb	%al, 191(%rsp)
	.loc 1 497 18
	movq	624(%rsp), %rax
	movl	240(%rax), %eax
	.loc 1 497 6
	testl	%eax, %eax
	je	.L77
	.loc 1 498 5
	leaq	304(%rsp), %rdx
	leaq	176(%rsp), %rcx
	leaq	176(%rsp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes128gcmsiv_ecb_enc_block@PLT
	jmp	.L78
.L77:
	.loc 1 500 5
	leaq	304(%rsp), %rdx
	leaq	176(%rsp), %rcx
	leaq	176(%rsp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes256gcmsiv_ecb_enc_block@PLT
.L78:
	.loc 1 503 7
	movq	16(%rbp), %rcx
	leaq	176(%rsp), %rax
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_memcmp@PLT
	.loc 1 503 6 discriminator 1
	testl	%eax, %eax
	je	.L79
	.loc 1 505 5
	movl	$505, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 506 12
	movl	$0, %eax
	jmp	.L80
.L79:
	.loc 1 509 10
	movl	$1, %eax
.L80:
	.loc 1 510 1
	movq	-8(%rbp), %rbx
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE159:
	.size	aead_aes_gcm_siv_asm_open_gather, .-aead_aes_gcm_siv_asm_open_gather
	.section	.data.rel.ro.local.aead_aes_128_gcm_siv_asm,"aw"
	.align 32
	.type	aead_aes_128_gcm_siv_asm, @object
	.size	aead_aes_128_gcm_siv_asm, 96
aead_aes_128_gcm_siv_asm:
	.byte	16
	.byte	12
	.byte	16
	.byte	16
	.value	3
	.zero	2
	.long	0
	.zero	4
	.quad	aead_aes_gcm_siv_asm_init
	.quad	0
	.quad	aead_aes_gcm_siv_asm_cleanup
	.quad	0
	.quad	aead_aes_gcm_siv_asm_seal_scatter
	.quad	aead_aes_gcm_siv_asm_open_gather
	.quad	0
	.quad	0
	.quad	0
	.quad	0
	.section	.data.rel.ro.local.aead_aes_256_gcm_siv_asm,"aw"
	.align 32
	.type	aead_aes_256_gcm_siv_asm, @object
	.size	aead_aes_256_gcm_siv_asm, 96
aead_aes_256_gcm_siv_asm:
	.byte	32
	.byte	12
	.byte	16
	.byte	16
	.value	4
	.zero	2
	.long	0
	.zero	4
	.quad	aead_aes_gcm_siv_asm_init
	.quad	0
	.quad	aead_aes_gcm_siv_asm_cleanup
	.quad	0
	.quad	aead_aes_gcm_siv_asm_seal_scatter
	.quad	aead_aes_gcm_siv_asm_open_gather
	.quad	0
	.quad	0
	.quad	0
	.quad	0
	.section	.text.aead_aes_gcm_siv_init,"ax",@progbits
	.type	aead_aes_gcm_siv_init, @function
aead_aes_gcm_siv_init:
.LFB160:
	.loc 1 571 66
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
	movq	%rcx, -48(%rbp)
	.loc 1 572 16
	movq	-40(%rbp), %rax
	salq	$3, %rax
	movq	%rax, -8(%rbp)
	.loc 1 574 6
	cmpq	$128, -8(%rbp)
	je	.L82
	.loc 1 574 23 discriminator 1
	cmpq	$256, -8(%rbp)
	je	.L82
	.loc 1 575 5
	movl	$575, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$102, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 576 12
	movl	$0, %eax
	jmp	.L83
.L82:
	.loc 1 579 6
	cmpq	$0, -48(%rbp)
	jne	.L84
	.loc 1 580 13
	movq	$16, -48(%rbp)
.L84:
	.loc 1 582 6
	cmpq	$16, -48(%rbp)
	je	.L85
	.loc 1 583 5
	movl	$583, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$116, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 584 12
	movl	$0, %eax
	jmp	.L83
.L85:
	.loc 1 587 32
	movq	-24(%rbp), %rax
	addq	$8, %rax
	movq	%rax, -16(%rbp)
	.loc 1 589 3
	movq	-16(%rbp), %rax
	movl	$264, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 591 3
	movq	-16(%rbp), %rax
	leaq	248(%rax), %rsi
	movq	-16(%rbp), %rax
	movq	-40(%rbp), %rcx
	movq	-32(%rbp), %rdx
	movq	%rcx, %r8
	movq	%rdx, %rcx
	movq	%rsi, %rdx
	movl	$0, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes_ctr_set_key@PLT
	.loc 1 593 34
	cmpq	$32, -40(%rbp)
	sete	%dl
	.loc 1 593 23
	movq	-16(%rbp), %rax
	movl	%edx, %ecx
	andl	$1, %ecx
	movzbl	256(%rax), %edx
	andl	$-2, %edx
	orl	%ecx, %edx
	movb	%dl, 256(%rax)
	.loc 1 594 16
	movq	-48(%rbp), %rax
	movl	%eax, %edx
	movq	-24(%rbp), %rax
	movb	%dl, 577(%rax)
	.loc 1 596 10
	movl	$1, %eax
.L83:
	.loc 1 597 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE160:
	.size	aead_aes_gcm_siv_init, .-aead_aes_gcm_siv_init
	.section	.text.aead_aes_gcm_siv_cleanup,"ax",@progbits
	.type	aead_aes_gcm_siv_cleanup, @function
aead_aes_gcm_siv_cleanup:
.LFB161:
	.loc 1 599 57
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	.loc 1 599 58
	nop
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE161:
	.size	aead_aes_gcm_siv_cleanup, .-aead_aes_gcm_siv_cleanup
	.section	.text.gcm_siv_crypt,"ax",@progbits
	.type	gcm_siv_crypt, @function
gcm_siv_crypt:
.LFB162:
	.loc 1 610 69
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$112, %rsp
	movq	%rdi, -72(%rbp)
	movq	%rsi, -80(%rbp)
	movq	%rdx, -88(%rbp)
	movq	%rcx, -96(%rbp)
	movq	%r8, -104(%rbp)
	movq	%r9, -112(%rbp)
	.loc 1 613 3
	movq	-96(%rbp), %rcx
	leaq	-48(%rbp), %rax
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 614 10
	movzbl	-33(%rbp), %eax
	.loc 1 614 15
	orl	$-128, %eax
	movb	%al, -33(%rbp)
.LBB5:
	.loc 1 616 15
	movq	$0, -8(%rbp)
	.loc 1 616 3
	jmp	.L88
.L92:
.LBB6:
	.loc 1 618 5
	movq	-112(%rbp), %rdx
	leaq	-64(%rbp), %rcx
	leaq	-48(%rbp), %rax
	movq	-104(%rbp), %r8
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	*%r8
.LVL0:
	.loc 1 619 34
	leaq	-48(%rbp), %rax
	movq	%rax, %rdi
	call	CRYPTO_load_u32_le
	.loc 1 619 5 discriminator 1
	leal	1(%rax), %edx
	leaq	-48(%rbp), %rax
	movl	%edx, %esi
	movq	%rax, %rdi
	call	CRYPTO_store_u32_le
	.loc 1 621 12
	movq	$16, -16(%rbp)
	.loc 1 622 16
	movq	-88(%rbp), %rax
	subq	-8(%rbp), %rax
	.loc 1 622 8
	cmpq	-16(%rbp), %rax
	jnb	.L89
	.loc 1 623 12
	movq	-88(%rbp), %rax
	subq	-8(%rbp), %rax
	movq	%rax, -16(%rbp)
.L89:
.LBB7:
	.loc 1 626 17
	movq	$0, -24(%rbp)
	.loc 1 626 5
	jmp	.L90
.L91:
	.loc 1 627 32
	leaq	-64(%rbp), %rdx
	movq	-24(%rbp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %esi
	.loc 1 627 46
	movq	-8(%rbp), %rdx
	movq	-24(%rbp), %rax
	addq	%rax, %rdx
	.loc 1 627 40
	movq	-80(%rbp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %ecx
	.loc 1 627 16
	movq	-8(%rbp), %rdx
	movq	-24(%rbp), %rax
	addq	%rax, %rdx
	.loc 1 627 10
	movq	-72(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 627 21
	xorl	%ecx, %esi
	movl	%esi, %edx
	movb	%dl, (%rax)
	.loc 1 626 35 discriminator 3
	addq	$1, -24(%rbp)
.L90:
	.loc 1 626 26 discriminator 1
	movq	-24(%rbp), %rax
	cmpq	-16(%rbp), %rax
	jb	.L91
.LBE7:
	.loc 1 630 10
	movq	-16(%rbp), %rax
	addq	%rax, -8(%rbp)
.L88:
.LBE6:
	.loc 1 616 30 discriminator 1
	movq	-8(%rbp), %rax
	cmpq	-88(%rbp), %rax
	jb	.L92
.LBE5:
	.loc 1 632 1
	nop
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE162:
	.size	gcm_siv_crypt, .-gcm_siv_crypt
	.section	.text.gcm_siv_polyval,"ax",@progbits
	.type	gcm_siv_polyval, @function
gcm_siv_polyval:
.LFB163:
	.loc 1 639 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$384, %rsp
	movq	%rdi, -344(%rbp)
	movq	%rsi, -352(%rbp)
	movq	%rdx, -360(%rbp)
	movq	%rcx, -368(%rbp)
	movq	%r8, -376(%rbp)
	movq	%r9, -384(%rbp)
	.loc 1 641 3
	movq	-384(%rbp), %rdx
	leaq	-304(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_POLYVAL_init@PLT
	.loc 1 643 3
	movq	-376(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rdx
	movq	-368(%rbp), %rcx
	leaq	-304(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_POLYVAL_update_blocks@PLT
	.loc 1 646 14
	movq	-376(%rbp), %rax
	andl	$15, %eax
	.loc 1 646 6
	testq	%rax, %rax
	je	.L94
	.loc 1 647 5
	leaq	-320(%rbp), %rax
	movl	$16, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 648 5
	movq	-376(%rbp), %rax
	andl	$15, %eax
	movq	%rax, %rdx
	.loc 1 648 40
	movq	-376(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rcx
	.loc 1 648 29
	movq	-368(%rbp), %rax
	addq	%rax, %rcx
	.loc 1 648 5
	leaq	-320(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 649 5
	leaq	-320(%rbp), %rcx
	leaq	-304(%rbp), %rax
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_POLYVAL_update_blocks@PLT
.L94:
	.loc 1 652 3
	movq	-360(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rdx
	movq	-352(%rbp), %rcx
	leaq	-304(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_POLYVAL_update_blocks@PLT
	.loc 1 653 14
	movq	-360(%rbp), %rax
	andl	$15, %eax
	.loc 1 653 6
	testq	%rax, %rax
	je	.L95
	.loc 1 654 5
	leaq	-320(%rbp), %rax
	movl	$16, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 655 5
	movq	-360(%rbp), %rax
	andl	$15, %eax
	movq	%rax, %rdx
	.loc 1 655 40
	movq	-360(%rbp), %rax
	andq	$-16, %rax
	movq	%rax, %rcx
	.loc 1 655 29
	movq	-352(%rbp), %rax
	addq	%rax, %rcx
	.loc 1 655 5
	leaq	-320(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 656 5
	leaq	-320(%rbp), %rcx
	leaq	-304(%rbp), %rax
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_POLYVAL_update_blocks@PLT
.L95:
	.loc 1 660 3
	movq	-376(%rbp), %rax
	leaq	0(,%rax,8), %rdx
	leaq	-336(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	CRYPTO_store_u64_le
	.loc 1 661 3
	movq	-360(%rbp), %rax
	leaq	0(,%rax,8), %rdx
	leaq	-336(%rbp), %rax
	addq	$8, %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	CRYPTO_store_u64_le
	.loc 1 662 3
	leaq	-336(%rbp), %rcx
	leaq	-304(%rbp), %rax
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_POLYVAL_update_blocks@PLT
	.loc 1 665 3
	movq	-344(%rbp), %rdx
	leaq	-304(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_POLYVAL_finish@PLT
.LBB8:
	.loc 1 666 15
	movq	$0, -8(%rbp)
	.loc 1 666 3
	jmp	.L96
.L97:
	.loc 1 667 12
	movq	-344(%rbp), %rdx
	movq	-8(%rbp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %esi
	.loc 1 667 24
	movq	16(%rbp), %rdx
	movq	-8(%rbp), %rax
	addq	%rdx, %rax
	movzbl	(%rax), %ecx
	.loc 1 667 12
	movq	-344(%rbp), %rdx
	movq	-8(%rbp), %rax
	addq	%rdx, %rax
	.loc 1 667 16
	xorl	%ecx, %esi
	movl	%esi, %edx
	movb	%dl, (%rax)
	.loc 1 666 59 discriminator 3
	addq	$1, -8(%rbp)
.L96:
	.loc 1 666 24 discriminator 1
	cmpq	$11, -8(%rbp)
	jbe	.L97
.LBE8:
	.loc 1 669 10
	movq	-344(%rbp), %rax
	addq	$15, %rax
	movzbl	(%rax), %edx
	movq	-344(%rbp), %rax
	addq	$15, %rax
	.loc 1 669 15
	andl	$127, %edx
	movb	%dl, (%rax)
	.loc 1 670 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE163:
	.size	gcm_siv_polyval, .-gcm_siv_polyval
	.section	.text.gcm_siv_keys,"ax",@progbits
	.type	gcm_siv_keys, @function
gcm_siv_keys:
.LFB164:
	.loc 1 686 79
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$144, %rsp
	movq	%rdi, -120(%rbp)
	movq	%rsi, -128(%rbp)
	movq	%rdx, -136(%rbp)
	.loc 1 687 24
	movq	-120(%rbp), %rax
	movq	%rax, -16(%rbp)
	.loc 1 689 52
	movq	-120(%rbp), %rax
	movzbl	256(%rax), %eax
	andl	$1, %eax
	.loc 1 689 56
	testb	%al, %al
	je	.L99
	.loc 1 689 56 is_stmt 0 discriminator 1
	movl	$6, %eax
	jmp	.L100
.L99:
	.loc 1 689 56 discriminator 2
	movl	$4, %eax
.L100:
	.loc 1 689 16 is_stmt 1 discriminator 4
	movq	%rax, -24(%rbp)
	.loc 1 692 3
	leaq	-96(%rbp), %rax
	movl	$4, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 693 3
	leaq	-96(%rbp), %rax
	addq	$4, %rax
	movq	-136(%rbp), %rcx
	movl	$12, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
.LBB9:
	.loc 1 695 15
	movq	$0, -8(%rbp)
	.loc 1 695 3
	jmp	.L101
.L102:
.LBB10:
	.loc 1 696 16
	movq	-8(%rbp), %rax
	movb	%al, -96(%rbp)
	.loc 1 699 16
	movq	-120(%rbp), %rax
	movq	248(%rax), %r8
	.loc 1 699 5
	movq	-16(%rbp), %rdx
	leaq	-112(%rbp), %rcx
	leaq	-96(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	*%r8
.LVL1:
	.loc 1 700 36
	movq	-8(%rbp), %rax
	leaq	0(,%rax,8), %rdx
	.loc 1 700 20
	leaq	-80(%rbp), %rax
	leaq	(%rax,%rdx), %rcx
	.loc 1 700 5
	leaq	-112(%rbp), %rax
	movl	$8, %edx
	movq	%rax, %rsi
	movq	%rcx, %rdi
	call	OPENSSL_memcpy
.LBE10:
	.loc 1 695 42 discriminator 3
	addq	$1, -8(%rbp)
.L101:
	.loc 1 695 24 discriminator 1
	movq	-8(%rbp), %rax
	cmpq	-24(%rbp), %rax
	jb	.L102
.LBE9:
	.loc 1 703 26
	movq	-128(%rbp), %rax
	.loc 1 703 3
	leaq	-80(%rbp), %rcx
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 713 58
	movq	-120(%rbp), %rax
	movzbl	256(%rax), %eax
	andl	$1, %eax
	.loc 1 712 3
	testb	%al, %al
	je	.L103
	.loc 1 712 3 is_stmt 0 discriminator 1
	movl	$32, %esi
	jmp	.L104
.L103:
	.loc 1 712 3 discriminator 2
	movl	$16, %esi
.L104:
	.loc 1 712 3 discriminator 4
	leaq	-80(%rbp), %rax
	addq	$16, %rax
	movq	-128(%rbp), %rdx
	addq	$264, %rdx
	movq	-128(%rbp), %rcx
	leaq	16(%rcx), %rdi
	movq	%rsi, %r8
	movq	%rax, %rcx
	movl	$0, %esi
	call	aws_lc_0_38_0_aes_ctr_set_key@PLT
	.loc 1 714 1 is_stmt 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE164:
	.size	gcm_siv_keys, .-gcm_siv_keys
	.section	.text.aead_aes_gcm_siv_seal_scatter,"ax",@progbits
	.type	aead_aes_gcm_siv_seal_scatter, @function
aead_aes_gcm_siv_seal_scatter:
.LFB165:
	.loc 1 720 60
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$368, %rsp
	movq	%rdi, -328(%rbp)
	movq	%rsi, -336(%rbp)
	movq	%rdx, -344(%rbp)
	movq	%rcx, -352(%rbp)
	movq	%r8, -360(%rbp)
	movq	%r9, -368(%rbp)
	.loc 1 721 38
	movq	-328(%rbp), %rax
	addq	$8, %rax
	movq	%rax, -8(%rbp)
	.loc 1 723 18
	movq	32(%rbp), %rax
	movq	%rax, -16(%rbp)
	.loc 1 724 18
	movq	64(%rbp), %rax
	movq	%rax, -24(%rbp)
	.loc 1 726 6
	cmpq	$-17, 32(%rbp)
	ja	.L106
	.loc 1 726 54 discriminator 1
	movabsq	$68719476736, %rax
	cmpq	-16(%rbp), %rax
	jb	.L106
	.loc 1 727 39
	movabsq	$2305843009213693951, %rax
	cmpq	-24(%rbp), %rax
	jnb	.L107
.L106:
	.loc 1 728 5
	movl	$728, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$117, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 729 12
	movl	$0, %eax
	jmp	.L111
.L107:
	.loc 1 732 6
	cmpq	$15, -360(%rbp)
	ja	.L109
	.loc 1 733 5
	movl	$733, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$103, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 734 12
	movl	$0, %eax
	jmp	.L111
.L109:
	.loc 1 737 6
	cmpq	$12, 16(%rbp)
	je	.L110
	.loc 1 738 5
	movl	$738, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$121, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 739 12
	movl	$0, %eax
	jmp	.L111
.L110:
	.loc 1 743 3
	movq	-368(%rbp), %rdx
	leaq	-304(%rbp), %rcx
	movq	-8(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	gcm_siv_keys
	.loc 1 746 3
	leaq	-304(%rbp), %r8
	movq	64(%rbp), %rdi
	movq	56(%rbp), %rcx
	movq	32(%rbp), %rdx
	movq	24(%rbp), %rsi
	leaq	-320(%rbp), %rax
	subq	$8, %rsp
	pushq	-368(%rbp)
	movq	%r8, %r9
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	gcm_siv_polyval
	addq	$16, %rsp
	.loc 1 747 7
	movq	-40(%rbp), %r8
	.loc 1 747 3
	leaq	-304(%rbp), %rax
	leaq	16(%rax), %rdx
	leaq	-320(%rbp), %rcx
	leaq	-320(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	*%r8
.LVL2:
	.loc 1 749 3
	movq	-40(%rbp), %rdi
	leaq	-304(%rbp), %rax
	leaq	16(%rax), %r8
	leaq	-320(%rbp), %rcx
	movq	32(%rbp), %rdx
	movq	24(%rbp), %rsi
	movq	-336(%rbp), %rax
	movq	%r8, %r9
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	gcm_siv_crypt
	.loc 1 751 3
	leaq	-320(%rbp), %rcx
	movq	-344(%rbp), %rax
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 752 16
	movq	-352(%rbp), %rax
	movq	$16, (%rax)
	.loc 1 754 10
	movl	$1, %eax
.L111:
	.loc 1 755 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE165:
	.size	aead_aes_gcm_siv_seal_scatter, .-aead_aes_gcm_siv_seal_scatter
	.section	.text.aead_aes_gcm_siv_open_gather,"ax",@progbits
	.type	aead_aes_gcm_siv_open_gather, @function
aead_aes_gcm_siv_open_gather:
.LFB166:
	.loc 1 762 56
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$368, %rsp
	movq	%rdi, -328(%rbp)
	movq	%rsi, -336(%rbp)
	movq	%rdx, -344(%rbp)
	movq	%rcx, -352(%rbp)
	movq	%r8, -360(%rbp)
	movq	%r9, -368(%rbp)
	.loc 1 763 18
	movq	40(%rbp), %rax
	movq	%rax, -8(%rbp)
	.loc 1 764 6
	movabsq	$2305843009213693951, %rax
	cmpq	-8(%rbp), %rax
	jnb	.L113
	.loc 1 765 5
	movl	$765, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$117, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 766 12
	movl	$0, %eax
	jmp	.L119
.L113:
	.loc 1 769 18
	movq	-368(%rbp), %rax
	movq	%rax, -16(%rbp)
	.loc 1 770 6
	cmpq	$16, 24(%rbp)
	jne	.L115
	.loc 1 770 50 discriminator 1
	movabsq	$68719476752, %rax
	cmpq	-16(%rbp), %rax
	jnb	.L116
.L115:
	.loc 1 772 5
	movl	$772, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 773 12
	movl	$0, %eax
	jmp	.L119
.L116:
	.loc 1 776 6
	cmpq	$12, -352(%rbp)
	je	.L117
	.loc 1 777 5
	movl	$777, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$121, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 778 12
	movl	$0, %eax
	jmp	.L119
.L117:
	.loc 1 781 38
	movq	-328(%rbp), %rax
	addq	$8, %rax
	movq	%rax, -24(%rbp)
	.loc 1 785 3
	movq	-344(%rbp), %rdx
	leaq	-304(%rbp), %rcx
	movq	-24(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	gcm_siv_keys
	.loc 1 787 3
	movq	-40(%rbp), %rdi
	leaq	-304(%rbp), %rax
	leaq	16(%rax), %r8
	movq	16(%rbp), %rcx
	movq	-368(%rbp), %rdx
	movq	-360(%rbp), %rsi
	movq	-336(%rbp), %rax
	movq	%r8, %r9
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	gcm_siv_crypt
	.loc 1 790 3
	leaq	-304(%rbp), %r8
	movq	40(%rbp), %rdi
	movq	32(%rbp), %rcx
	movq	-368(%rbp), %rdx
	movq	-336(%rbp), %rsi
	leaq	-320(%rbp), %rax
	subq	$8, %rsp
	pushq	-344(%rbp)
	movq	%r8, %r9
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	gcm_siv_polyval
	addq	$16, %rsp
	.loc 1 791 7
	movq	-40(%rbp), %r8
	.loc 1 791 3
	leaq	-304(%rbp), %rax
	leaq	16(%rax), %rdx
	leaq	-320(%rbp), %rcx
	leaq	-320(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	*%r8
.LVL3:
	.loc 1 793 7
	movq	16(%rbp), %rcx
	leaq	-320(%rbp), %rax
	movl	$16, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_memcmp@PLT
	.loc 1 793 6 discriminator 1
	testl	%eax, %eax
	je	.L118
	.loc 1 794 5
	movl	$794, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 795 12
	movl	$0, %eax
	jmp	.L119
.L118:
	.loc 1 798 10
	movl	$1, %eax
.L119:
	.loc 1 799 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE166:
	.size	aead_aes_gcm_siv_open_gather, .-aead_aes_gcm_siv_open_gather
	.section	.data.rel.ro.local.aead_aes_128_gcm_siv,"aw"
	.align 32
	.type	aead_aes_128_gcm_siv, @object
	.size	aead_aes_128_gcm_siv, 96
aead_aes_128_gcm_siv:
	.byte	16
	.byte	12
	.byte	16
	.byte	16
	.value	3
	.zero	2
	.long	0
	.zero	4
	.quad	aead_aes_gcm_siv_init
	.quad	0
	.quad	aead_aes_gcm_siv_cleanup
	.quad	0
	.quad	aead_aes_gcm_siv_seal_scatter
	.quad	aead_aes_gcm_siv_open_gather
	.quad	0
	.quad	0
	.quad	0
	.quad	0
	.section	.data.rel.ro.local.aead_aes_256_gcm_siv,"aw"
	.align 32
	.type	aead_aes_256_gcm_siv, @object
	.size	aead_aes_256_gcm_siv, 96
aead_aes_256_gcm_siv:
	.byte	32
	.byte	12
	.byte	16
	.byte	16
	.value	4
	.zero	2
	.long	0
	.zero	4
	.quad	aead_aes_gcm_siv_init
	.quad	0
	.quad	aead_aes_gcm_siv_cleanup
	.quad	0
	.quad	aead_aes_gcm_siv_seal_scatter
	.quad	aead_aes_gcm_siv_open_gather
	.quad	0
	.quad	0
	.quad	0
	.quad	0
	.section	.text.aws_lc_0_38_0_EVP_aead_aes_128_gcm_siv,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_aead_aes_128_gcm_siv
	.type	aws_lc_0_38_0_EVP_aead_aes_128_gcm_siv, @function
aws_lc_0_38_0_EVP_aead_aes_128_gcm_siv:
.LFB167:
	.loc 1 843 48
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 1 844 7
	call	CRYPTO_is_AVX_capable
	.loc 1 844 6 discriminator 1
	testl	%eax, %eax
	je	.L121
	.loc 1 844 34 discriminator 1
	call	CRYPTO_is_AESNI_capable
	.loc 1 844 31 discriminator 1
	testl	%eax, %eax
	je	.L121
	.loc 1 845 12
	leaq	aead_aes_128_gcm_siv_asm(%rip), %rax
	jmp	.L122
.L121:
	.loc 1 847 10
	leaq	aead_aes_128_gcm_siv(%rip), %rax
.L122:
	.loc 1 848 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE167:
	.size	aws_lc_0_38_0_EVP_aead_aes_128_gcm_siv, .-aws_lc_0_38_0_EVP_aead_aes_128_gcm_siv
	.section	.text.aws_lc_0_38_0_EVP_aead_aes_256_gcm_siv,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_aead_aes_256_gcm_siv
	.type	aws_lc_0_38_0_EVP_aead_aes_256_gcm_siv, @function
aws_lc_0_38_0_EVP_aead_aes_256_gcm_siv:
.LFB168:
	.loc 1 850 48
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 1 851 7
	call	CRYPTO_is_AVX_capable
	.loc 1 851 6 discriminator 1
	testl	%eax, %eax
	je	.L124
	.loc 1 851 34 discriminator 1
	call	CRYPTO_is_AESNI_capable
	.loc 1 851 31 discriminator 1
	testl	%eax, %eax
	je	.L124
	.loc 1 852 12
	leaq	aead_aes_256_gcm_siv_asm(%rip), %rax
	jmp	.L125
.L124:
	.loc 1 854 10
	leaq	aead_aes_256_gcm_siv(%rip), %rax
.L125:
	.loc 1 855 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE168:
	.size	aws_lc_0_38_0_EVP_aead_aes_256_gcm_siv, .-aws_lc_0_38_0_EVP_aead_aes_256_gcm_siv
	.section	.text.aws_lc_0_38_0_x86_64_assembly_implementation_FOR_TESTING,"ax",@progbits
	.globl	aws_lc_0_38_0_x86_64_assembly_implementation_FOR_TESTING
	.type	aws_lc_0_38_0_x86_64_assembly_implementation_FOR_TESTING, @function
aws_lc_0_38_0_x86_64_assembly_implementation_FOR_TESTING:
.LFB169:
	.loc 1 857 54
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 1 858 7
	call	CRYPTO_is_AVX_capable
	.loc 1 858 6 discriminator 1
	testl	%eax, %eax
	je	.L127
	.loc 1 858 34 discriminator 1
	call	CRYPTO_is_AESNI_capable
	.loc 1 858 31 discriminator 1
	testl	%eax, %eax
	je	.L127
	.loc 1 859 12
	movl	$1, %eax
	jmp	.L128
.L127:
	.loc 1 861 10
	movl	$0, %eax
.L128:
	.loc 1 862 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE169:
	.size	aws_lc_0_38_0_x86_64_assembly_implementation_FOR_TESTING, .-aws_lc_0_38_0_x86_64_assembly_implementation_FOR_TESTING
	.section	.rodata.__PRETTY_FUNCTION__.0,"a"
	.align 16
	.type	__PRETTY_FUNCTION__.0, @object
	.size	__PRETTY_FUNCTION__.0, 26
__PRETTY_FUNCTION__.0:
	.string	"aead_aes_gcm_siv_asm_init"
	.text
.Letext0:
	.file 4 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 5 "/usr/include/bits/types.h"
	.file 6 "/usr/include/bits/stdint-uintn.h"
	.file 7 "/usr/include/stdint.h"
	.file 8 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/bytestring.h"
	.file 9 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.file 10 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/../fipsmodule/cipher/internal.h"
	.file 11 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/aead.h"
	.file 12 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/aes.h"
	.file 13 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/../fipsmodule/cipher/../modes/internal.h"
	.file 14 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/mem.h"
	.file 15 "/usr/include/string.h"
	.file 16 "/usr/include/assert.h"
	.file 17 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/err.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x1c47
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF194
	.byte	0xc
	.long	.LASF195
	.long	.LASF196
	.long	.Ldebug_ranges0+0
	.quad	0
	.long	.Ldebug_line0
	.uleb128 0x2
	.byte	0x8
	.byte	0x5
	.long	.LASF0
	.uleb128 0x3
	.long	.LASF8
	.byte	0x4
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
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF7
	.uleb128 0x3
	.long	.LASF9
	.byte	0x5
	.byte	0x26
	.byte	0x17
	.long	0x5d
	.uleb128 0x2
	.byte	0x2
	.byte	0x5
	.long	.LASF10
	.uleb128 0x3
	.long	.LASF11
	.byte	0x5
	.byte	0x28
	.byte	0x1c
	.long	0x64
	.uleb128 0x3
	.long	.LASF12
	.byte	0x5
	.byte	0x2a
	.byte	0x16
	.long	0x6b
	.uleb128 0x3
	.long	.LASF13
	.byte	0x5
	.byte	0x2d
	.byte	0x1b
	.long	0x41
	.uleb128 0x6
	.byte	0x8
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF14
	.uleb128 0x4
	.long	0xb2
	.uleb128 0x3
	.long	.LASF15
	.byte	0x6
	.byte	0x18
	.byte	0x13
	.long	0x79
	.uleb128 0x4
	.long	0xbe
	.uleb128 0x3
	.long	.LASF16
	.byte	0x6
	.byte	0x19
	.byte	0x14
	.long	0x8c
	.uleb128 0x3
	.long	.LASF17
	.byte	0x6
	.byte	0x1a
	.byte	0x14
	.long	0x98
	.uleb128 0x4
	.long	0xdb
	.uleb128 0x3
	.long	.LASF18
	.byte	0x6
	.byte	0x1b
	.byte	0x14
	.long	0xa4
	.uleb128 0x4
	.long	0xec
	.uleb128 0x3
	.long	.LASF19
	.byte	0x7
	.byte	0x4f
	.byte	0x1b
	.long	0x41
	.uleb128 0x4
	.long	0xfd
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF20
	.uleb128 0x7
	.byte	0x8
	.long	0x11b
	.uleb128 0x8
	.uleb128 0x9
	.string	"CBB"
	.byte	0x9
	.value	0x194
	.byte	0x17
	.long	0x129
	.uleb128 0xa
	.long	.LASF23
	.byte	0x30
	.byte	0x8
	.value	0x1be
	.byte	0x8
	.long	0x160
	.uleb128 0xb
	.long	.LASF21
	.byte	0x8
	.value	0x1c0
	.byte	0x8
	.long	0x498
	.byte	0
	.uleb128 0xb
	.long	.LASF22
	.byte	0x8
	.value	0x1c3
	.byte	0x8
	.long	0xb2
	.byte	0x8
	.uleb128 0xc
	.string	"u"
	.byte	0x8
	.value	0x1c7
	.byte	0x5
	.long	0x473
	.byte	0x10
	.byte	0
	.uleb128 0x9
	.string	"CBS"
	.byte	0x9
	.value	0x195
	.byte	0x17
	.long	0x16d
	.uleb128 0xd
	.long	.LASF24
	.byte	0x10
	.byte	0x8
	.byte	0x28
	.byte	0x8
	.long	0x195
	.uleb128 0xe
	.long	.LASF25
	.byte	0x8
	.byte	0x29
	.byte	0x12
	.long	0x3bc
	.byte	0
	.uleb128 0xf
	.string	"len"
	.byte	0x8
	.byte	0x2a
	.byte	0xa
	.long	0x30
	.byte	0x8
	.byte	0
	.uleb128 0x10
	.long	.LASF26
	.byte	0x9
	.value	0x1a6
	.byte	0x1c
	.long	0x1a7
	.uleb128 0x4
	.long	0x195
	.uleb128 0xd
	.long	.LASF27
	.byte	0x60
	.byte	0xa
	.byte	0x71
	.byte	0x8
	.long	0x285
	.uleb128 0xe
	.long	.LASF28
	.byte	0xa
	.byte	0x72
	.byte	0xb
	.long	0xbe
	.byte	0
	.uleb128 0xe
	.long	.LASF29
	.byte	0xa
	.byte	0x73
	.byte	0xb
	.long	0xbe
	.byte	0x1
	.uleb128 0xe
	.long	.LASF30
	.byte	0xa
	.byte	0x74
	.byte	0xb
	.long	0xbe
	.byte	0x2
	.uleb128 0xe
	.long	.LASF31
	.byte	0xa
	.byte	0x75
	.byte	0xb
	.long	0xbe
	.byte	0x3
	.uleb128 0xe
	.long	.LASF32
	.byte	0xa
	.byte	0x76
	.byte	0xc
	.long	0xcf
	.byte	0x4
	.uleb128 0xe
	.long	.LASF33
	.byte	0xa
	.byte	0x77
	.byte	0x7
	.long	0x48
	.byte	0x8
	.uleb128 0xe
	.long	.LASF34
	.byte	0xa
	.byte	0x7b
	.byte	0x9
	.long	0x6a5
	.byte	0x10
	.uleb128 0xe
	.long	.LASF35
	.byte	0xa
	.byte	0x7d
	.byte	0x9
	.long	0x6ce
	.byte	0x18
	.uleb128 0xe
	.long	.LASF36
	.byte	0xa
	.byte	0x7f
	.byte	0xa
	.long	0x6df
	.byte	0x20
	.uleb128 0xe
	.long	.LASF37
	.byte	0xa
	.byte	0x81
	.byte	0x9
	.long	0x72d
	.byte	0x28
	.uleb128 0xe
	.long	.LASF38
	.byte	0xa
	.byte	0x86
	.byte	0x9
	.long	0x77e
	.byte	0x30
	.uleb128 0xe
	.long	.LASF39
	.byte	0xa
	.byte	0x8c
	.byte	0x9
	.long	0x7c0
	.byte	0x38
	.uleb128 0xe
	.long	.LASF40
	.byte	0xa
	.byte	0x91
	.byte	0x9
	.long	0x7e5
	.byte	0x40
	.uleb128 0xe
	.long	.LASF41
	.byte	0xa
	.byte	0x94
	.byte	0xc
	.long	0x804
	.byte	0x48
	.uleb128 0xe
	.long	.LASF42
	.byte	0xa
	.byte	0x97
	.byte	0x9
	.long	0x81e
	.byte	0x50
	.uleb128 0xe
	.long	.LASF43
	.byte	0xa
	.byte	0x99
	.byte	0x9
	.long	0x83e
	.byte	0x58
	.byte	0
	.uleb128 0x10
	.long	.LASF44
	.byte	0x9
	.value	0x1a7
	.byte	0x20
	.long	0x297
	.uleb128 0x4
	.long	0x285
	.uleb128 0x11
	.long	.LASF45
	.value	0x248
	.byte	0xb
	.byte	0xdd
	.byte	0x8
	.long	0x2dc
	.uleb128 0xe
	.long	.LASF46
	.byte	0xb
	.byte	0xde
	.byte	0x13
	.long	0x320
	.byte	0
	.uleb128 0xe
	.long	.LASF47
	.byte	0xb
	.byte	0xdf
	.byte	0x1f
	.long	0x2dc
	.byte	0x8
	.uleb128 0x12
	.long	.LASF48
	.byte	0xb
	.byte	0xe0
	.byte	0xb
	.long	0xbe
	.value	0x240
	.uleb128 0x12
	.long	.LASF41
	.byte	0xb
	.byte	0xe3
	.byte	0xb
	.long	0xbe
	.value	0x241
	.byte	0
	.uleb128 0x13
	.long	.LASF197
	.value	0x238
	.byte	0xb
	.byte	0xd5
	.byte	0x7
	.long	0x30f
	.uleb128 0x14
	.long	.LASF49
	.byte	0xb
	.byte	0xd6
	.byte	0xb
	.long	0x30f
	.uleb128 0x14
	.long	.LASF50
	.byte	0xb
	.byte	0xd7
	.byte	0xc
	.long	0xec
	.uleb128 0x15
	.string	"ptr"
	.byte	0xb
	.byte	0xd8
	.byte	0x9
	.long	0xb0
	.byte	0
	.uleb128 0x16
	.long	0xbe
	.long	0x320
	.uleb128 0x17
	.long	0x41
	.value	0x233
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x1a2
	.uleb128 0x18
	.long	.LASF198
	.byte	0x7
	.byte	0x4
	.long	0x6b
	.byte	0xb
	.value	0x1b6
	.byte	0x6
	.long	0x346
	.uleb128 0x19
	.long	.LASF51
	.byte	0
	.uleb128 0x19
	.long	.LASF52
	.byte	0x1
	.byte	0
	.uleb128 0x16
	.long	0xbe
	.long	0x356
	.uleb128 0x1a
	.long	0x41
	.byte	0xf
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xb9
	.uleb128 0x16
	.long	0xbe
	.long	0x36c
	.uleb128 0x1a
	.long	0x41
	.byte	0x7f
	.byte	0
	.uleb128 0x2
	.byte	0x8
	.byte	0x4
	.long	.LASF53
	.uleb128 0xd
	.long	.LASF54
	.byte	0xf4
	.byte	0xc
	.byte	0x48
	.byte	0x8
	.long	0x39b
	.uleb128 0xe
	.long	.LASF55
	.byte	0xc
	.byte	0x49
	.byte	0xc
	.long	0x39b
	.byte	0
	.uleb128 0xe
	.long	.LASF56
	.byte	0xc
	.byte	0x4a
	.byte	0xc
	.long	0x6b
	.byte	0xf0
	.byte	0
	.uleb128 0x16
	.long	0xdb
	.long	0x3ab
	.uleb128 0x1a
	.long	0x41
	.byte	0x3b
	.byte	0
	.uleb128 0x3
	.long	.LASF57
	.byte	0xc
	.byte	0x4c
	.byte	0x1b
	.long	0x373
	.uleb128 0x4
	.long	0x3ab
	.uleb128 0x7
	.byte	0x8
	.long	0xca
	.uleb128 0xa
	.long	.LASF58
	.byte	0x20
	.byte	0x8
	.value	0x1a4
	.byte	0x8
	.long	0x41d
	.uleb128 0xc
	.string	"buf"
	.byte	0x8
	.value	0x1a5
	.byte	0xc
	.long	0x41d
	.byte	0
	.uleb128 0xc
	.string	"len"
	.byte	0x8
	.value	0x1a7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0xc
	.string	"cap"
	.byte	0x8
	.value	0x1a9
	.byte	0xa
	.long	0x30
	.byte	0x10
	.uleb128 0x1b
	.long	.LASF59
	.byte	0x8
	.value	0x1ac
	.byte	0xc
	.long	0x6b
	.byte	0x4
	.byte	0x1
	.byte	0x1f
	.byte	0x18
	.uleb128 0x1b
	.long	.LASF60
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
	.long	0xbe
	.uleb128 0xa
	.long	.LASF61
	.byte	0x18
	.byte	0x8
	.value	0x1b2
	.byte	0x8
	.long	0x46d
	.uleb128 0xb
	.long	.LASF62
	.byte	0x8
	.value	0x1b4
	.byte	0x19
	.long	0x46d
	.byte	0
	.uleb128 0xb
	.long	.LASF63
	.byte	0x8
	.value	0x1b7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0xb
	.long	.LASF64
	.byte	0x8
	.value	0x1ba
	.byte	0xb
	.long	0xbe
	.byte	0x10
	.uleb128 0x1b
	.long	.LASF65
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
	.long	0x3c2
	.uleb128 0x1c
	.byte	0x20
	.byte	0x8
	.value	0x1c4
	.byte	0x3
	.long	0x498
	.uleb128 0x1d
	.long	.LASF62
	.byte	0x8
	.value	0x1c5
	.byte	0x1a
	.long	0x3c2
	.uleb128 0x1d
	.long	.LASF21
	.byte	0x8
	.value	0x1c6
	.byte	0x19
	.long	0x423
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x11c
	.uleb128 0x2
	.byte	0x10
	.byte	0x5
	.long	.LASF66
	.uleb128 0x2
	.byte	0x10
	.byte	0x7
	.long	.LASF67
	.uleb128 0x2
	.byte	0x1
	.byte	0x2
	.long	.LASF68
	.uleb128 0x16
	.long	0xdb
	.long	0x4c3
	.uleb128 0x1a
	.long	0x41
	.byte	0x3
	.byte	0
	.uleb128 0x1e
	.long	.LASF161
	.byte	0x3
	.byte	0x28
	.byte	0x11
	.long	0x4b3
	.uleb128 0x3
	.long	.LASF69
	.byte	0xd
	.byte	0x50
	.byte	0x10
	.long	0x4db
	.uleb128 0x7
	.byte	0x8
	.long	0x4e1
	.uleb128 0x1f
	.long	0x4f6
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x4f6
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x3b7
	.uleb128 0x4
	.long	0x4f6
	.uleb128 0x3
	.long	.LASF70
	.byte	0xd
	.byte	0x65
	.byte	0x10
	.long	0x50d
	.uleb128 0x7
	.byte	0x8
	.long	0x513
	.uleb128 0x1f
	.long	0x532
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x4f6
	.uleb128 0x20
	.long	0x3bc
	.byte	0
	.uleb128 0x21
	.byte	0x10
	.byte	0xd
	.byte	0x85
	.byte	0x9
	.long	0x554
	.uleb128 0xf
	.string	"hi"
	.byte	0xd
	.byte	0x85
	.byte	0x1b
	.long	0xec
	.byte	0
	.uleb128 0xf
	.string	"lo"
	.byte	0xd
	.byte	0x85
	.byte	0x1e
	.long	0xec
	.byte	0x8
	.byte	0
	.uleb128 0x3
	.long	.LASF71
	.byte	0xd
	.byte	0x85
	.byte	0x24
	.long	0x532
	.uleb128 0x4
	.long	0x554
	.uleb128 0x3
	.long	.LASF72
	.byte	0xd
	.byte	0x89
	.byte	0x10
	.long	0x571
	.uleb128 0x7
	.byte	0x8
	.long	0x577
	.uleb128 0x1f
	.long	0x587
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x587
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x560
	.uleb128 0x3
	.long	.LASF73
	.byte	0xd
	.byte	0x8e
	.byte	0x10
	.long	0x599
	.uleb128 0x7
	.byte	0x8
	.long	0x59f
	.uleb128 0x1f
	.long	0x5b9
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x587
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x11
	.long	.LASF74
	.value	0x120
	.byte	0xd
	.byte	0x91
	.byte	0x10
	.long	0x610
	.uleb128 0xe
	.long	.LASF75
	.byte	0xd
	.byte	0x96
	.byte	0x8
	.long	0x610
	.byte	0
	.uleb128 0x12
	.long	.LASF76
	.byte	0xd
	.byte	0x97
	.byte	0xe
	.long	0x565
	.value	0x100
	.uleb128 0x12
	.long	.LASF77
	.byte	0xd
	.byte	0x98
	.byte	0xe
	.long	0x58d
	.value	0x108
	.uleb128 0x12
	.long	.LASF78
	.byte	0xd
	.byte	0x9a
	.byte	0xe
	.long	0x4cf
	.value	0x110
	.uleb128 0x22
	.long	.LASF79
	.byte	0xd
	.byte	0x9e
	.byte	0xc
	.long	0x6b
	.byte	0x4
	.byte	0x1
	.byte	0x1f
	.value	0x118
	.byte	0
	.uleb128 0x16
	.long	0x554
	.long	0x620
	.uleb128 0x1a
	.long	0x41
	.byte	0xf
	.byte	0
	.uleb128 0x3
	.long	.LASF80
	.byte	0xd
	.byte	0x9f
	.byte	0x3
	.long	0x5b9
	.uleb128 0x7
	.byte	0x8
	.long	0x3ab
	.uleb128 0x23
	.long	.LASF81
	.value	0x120
	.byte	0x10
	.byte	0xd
	.value	0x1db
	.byte	0x8
	.long	0x67c
	.uleb128 0xc
	.string	"S"
	.byte	0xd
	.value	0x1dc
	.byte	0xb
	.long	0x346
	.byte	0
	.uleb128 0x24
	.long	.LASF75
	.byte	0xd
	.value	0x1df
	.byte	0x14
	.long	0x610
	.byte	0x10
	.byte	0x10
	.uleb128 0x25
	.long	.LASF76
	.byte	0xd
	.value	0x1e0
	.byte	0xe
	.long	0x565
	.value	0x110
	.uleb128 0x25
	.long	.LASF77
	.byte	0xd
	.value	0x1e1
	.byte	0xe
	.long	0x58d
	.value	0x118
	.byte	0
	.uleb128 0x4
	.long	0x632
	.uleb128 0x26
	.long	0x48
	.long	0x69f
	.uleb128 0x20
	.long	0x69f
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x285
	.uleb128 0x7
	.byte	0x8
	.long	0x681
	.uleb128 0x26
	.long	0x48
	.long	0x6ce
	.uleb128 0x20
	.long	0x69f
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x326
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x6ab
	.uleb128 0x1f
	.long	0x6df
	.uleb128 0x20
	.long	0x69f
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x6d4
	.uleb128 0x26
	.long	0x48
	.long	0x721
	.uleb128 0x20
	.long	0x721
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x727
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x292
	.uleb128 0x7
	.byte	0x8
	.long	0x30
	.uleb128 0x7
	.byte	0x8
	.long	0x6e5
	.uleb128 0x26
	.long	0x48
	.long	0x77e
	.uleb128 0x20
	.long	0x721
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x727
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x733
	.uleb128 0x26
	.long	0x48
	.long	0x7c0
	.uleb128 0x20
	.long	0x721
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x784
	.uleb128 0x26
	.long	0x48
	.long	0x7df
	.uleb128 0x20
	.long	0x721
	.uleb128 0x20
	.long	0x7df
	.uleb128 0x20
	.long	0x727
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x3bc
	.uleb128 0x7
	.byte	0x8
	.long	0x7c6
	.uleb128 0x26
	.long	0x30
	.long	0x804
	.uleb128 0x20
	.long	0x721
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x7eb
	.uleb128 0x26
	.long	0x48
	.long	0x81e
	.uleb128 0x20
	.long	0x721
	.uleb128 0x20
	.long	0x498
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x80a
	.uleb128 0x26
	.long	0x48
	.long	0x838
	.uleb128 0x20
	.long	0x721
	.uleb128 0x20
	.long	0x838
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x160
	.uleb128 0x7
	.byte	0x8
	.long	0x824
	.uleb128 0x27
	.long	.LASF82
	.value	0x100
	.byte	0x10
	.byte	0x1
	.byte	0x27
	.byte	0x8
	.long	0x86f
	.uleb128 0x28
	.string	"key"
	.byte	0x1
	.byte	0x28
	.byte	0x17
	.long	0x874
	.byte	0x10
	.byte	0
	.uleb128 0xe
	.long	.LASF83
	.byte	0x1
	.byte	0x29
	.byte	0x7
	.long	0x48
	.byte	0xf0
	.byte	0
	.uleb128 0x4
	.long	0x844
	.uleb128 0x16
	.long	0xbe
	.long	0x884
	.uleb128 0x1a
	.long	0x41
	.byte	0xef
	.byte	0
	.uleb128 0x29
	.long	.LASF84
	.byte	0x1
	.value	0x200
	.byte	0x17
	.long	0x1a2
	.uleb128 0x9
	.byte	0x3
	.quad	aead_aes_128_gcm_siv_asm
	.uleb128 0x29
	.long	.LASF85
	.byte	0x1
	.value	0x214
	.byte	0x17
	.long	0x1a2
	.uleb128 0x9
	.byte	0x3
	.quad	aead_aes_256_gcm_siv_asm
	.uleb128 0x1c
	.byte	0xf8
	.byte	0x1
	.value	0x22b
	.byte	0x3
	.long	0x8d6
	.uleb128 0x1d
	.long	.LASF86
	.byte	0x1
	.value	0x22c
	.byte	0xc
	.long	0x36c
	.uleb128 0x2a
	.string	"ks"
	.byte	0x1
	.value	0x22d
	.byte	0xd
	.long	0x3ab
	.byte	0
	.uleb128 0x2b
	.long	.LASF87
	.value	0x108
	.byte	0x1
	.value	0x22a
	.byte	0x8
	.long	0x913
	.uleb128 0xc
	.string	"ks"
	.byte	0x1
	.value	0x22e
	.byte	0x5
	.long	0x8b2
	.byte	0
	.uleb128 0xb
	.long	.LASF88
	.byte	0x1
	.value	0x22f
	.byte	0xe
	.long	0x4cf
	.byte	0xf8
	.uleb128 0x2c
	.long	.LASF89
	.byte	0x1
	.value	0x230
	.byte	0xc
	.long	0x6b
	.byte	0x4
	.byte	0x1
	.byte	0x1f
	.value	0x100
	.byte	0
	.uleb128 0x4
	.long	0x8d6
	.uleb128 0x1c
	.byte	0xf8
	.byte	0x1
	.value	0x2a3
	.byte	0x3
	.long	0x93c
	.uleb128 0x1d
	.long	.LASF86
	.byte	0x1
	.value	0x2a4
	.byte	0xc
	.long	0x36c
	.uleb128 0x2a
	.string	"ks"
	.byte	0x1
	.value	0x2a5
	.byte	0xd
	.long	0x3ab
	.byte	0
	.uleb128 0x2b
	.long	.LASF90
	.value	0x110
	.byte	0x1
	.value	0x2a1
	.byte	0x8
	.long	0x977
	.uleb128 0xb
	.long	.LASF91
	.byte	0x1
	.value	0x2a2
	.byte	0xb
	.long	0x346
	.byte	0
	.uleb128 0xb
	.long	.LASF92
	.byte	0x1
	.value	0x2a6
	.byte	0x5
	.long	0x918
	.byte	0x10
	.uleb128 0x25
	.long	.LASF93
	.byte	0x1
	.value	0x2a7
	.byte	0xe
	.long	0x4cf
	.value	0x108
	.byte	0
	.uleb128 0x29
	.long	.LASF94
	.byte	0x1
	.value	0x321
	.byte	0x17
	.long	0x1a2
	.uleb128 0x9
	.byte	0x3
	.quad	aead_aes_128_gcm_siv
	.uleb128 0x29
	.long	.LASF95
	.byte	0x1
	.value	0x335
	.byte	0x17
	.long	0x1a2
	.uleb128 0x9
	.byte	0x3
	.quad	aead_aes_256_gcm_siv
	.uleb128 0x2d
	.long	.LASF96
	.byte	0xd
	.value	0x1ee
	.byte	0x6
	.long	0x9bd
	.uleb128 0x20
	.long	0x9bd
	.uleb128 0x20
	.long	0x41d
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x67c
	.uleb128 0x2d
	.long	.LASF97
	.byte	0xd
	.value	0x1ea
	.byte	0x6
	.long	0x9e0
	.uleb128 0x20
	.long	0x9e0
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x632
	.uleb128 0x2d
	.long	.LASF98
	.byte	0xd
	.value	0x1e5
	.byte	0x6
	.long	0x9fe
	.uleb128 0x20
	.long	0x9e0
	.uleb128 0x20
	.long	0x3bc
	.byte	0
	.uleb128 0x2e
	.long	.LASF99
	.byte	0xa
	.byte	0xc5
	.byte	0xa
	.long	0x501
	.long	0xa28
	.uleb128 0x20
	.long	0x62c
	.uleb128 0x20
	.long	0xa28
	.uleb128 0x20
	.long	0xa2e
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x620
	.uleb128 0x7
	.byte	0x8
	.long	0x4cf
	.uleb128 0x2e
	.long	.LASF100
	.byte	0xe
	.byte	0x74
	.byte	0x14
	.long	0x48
	.long	0xa54
	.uleb128 0x20
	.long	0x115
	.uleb128 0x20
	.long	0x115
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x2f
	.long	.LASF101
	.byte	0x1
	.byte	0x9b
	.byte	0xd
	.long	0xa7f
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0xa7f
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x86f
	.uleb128 0x2f
	.long	.LASF102
	.byte	0x1
	.byte	0x94
	.byte	0xd
	.long	0xab0
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0xa7f
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x2f
	.long	.LASF103
	.byte	0x1
	.byte	0x7f
	.byte	0xd
	.long	0xac7
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x3bc
	.byte	0
	.uleb128 0x2f
	.long	.LASF104
	.byte	0x1
	.byte	0xe1
	.byte	0xd
	.long	0xaed
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0xa7f
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x2f
	.long	.LASF105
	.byte	0x1
	.byte	0xd3
	.byte	0xd
	.long	0xb13
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0xa7f
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x2f
	.long	.LASF106
	.byte	0x1
	.byte	0xb7
	.byte	0xd
	.long	0xb34
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0xb34
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xf8
	.uleb128 0x2f
	.long	.LASF107
	.byte	0x1
	.byte	0xda
	.byte	0xd
	.long	0xb60
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0xa7f
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x2f
	.long	.LASF108
	.byte	0x1
	.byte	0xcc
	.byte	0xd
	.long	0xb86
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0xa7f
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x2f
	.long	.LASF109
	.byte	0x1
	.byte	0xb1
	.byte	0xd
	.long	0xba7
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0xb34
	.byte	0
	.uleb128 0x2f
	.long	.LASF110
	.byte	0x1
	.byte	0xaa
	.byte	0xd
	.long	0xbc3
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0xbc3
	.uleb128 0x20
	.long	0x3bc
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xec
	.uleb128 0x2f
	.long	.LASF111
	.byte	0x1
	.byte	0xa5
	.byte	0xd
	.long	0xbe5
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0xbc3
	.uleb128 0x20
	.long	0x3bc
	.byte	0
	.uleb128 0x2f
	.long	.LASF112
	.byte	0x1
	.byte	0xc3
	.byte	0xd
	.long	0xc01
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0xa7f
	.byte	0
	.uleb128 0x2f
	.long	.LASF113
	.byte	0x1
	.byte	0xbd
	.byte	0xd
	.long	0xc1d
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0xa7f
	.byte	0
	.uleb128 0x2e
	.long	.LASF114
	.byte	0xf
	.byte	0x2b
	.byte	0xe
	.long	0xb0
	.long	0xc3d
	.uleb128 0x20
	.long	0xb0
	.uleb128 0x20
	.long	0x115
	.uleb128 0x20
	.long	0x41
	.byte	0
	.uleb128 0x2e
	.long	.LASF115
	.byte	0xf
	.byte	0x3d
	.byte	0xe
	.long	0xb0
	.long	0xc5d
	.uleb128 0x20
	.long	0xb0
	.uleb128 0x20
	.long	0x48
	.uleb128 0x20
	.long	0x41
	.byte	0
	.uleb128 0x2f
	.long	.LASF116
	.byte	0x1
	.byte	0x76
	.byte	0xd
	.long	0xc7e
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.byte	0
	.uleb128 0x2f
	.long	.LASF117
	.byte	0x1
	.byte	0x85
	.byte	0xd
	.long	0xc9f
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x30
	.uleb128 0x20
	.long	0x41d
	.byte	0
	.uleb128 0x2f
	.long	.LASF118
	.byte	0x1
	.byte	0x7b
	.byte	0xd
	.long	0xcb6
	.uleb128 0x20
	.long	0x41d
	.uleb128 0x20
	.long	0x3bc
	.byte	0
	.uleb128 0x2f
	.long	.LASF119
	.byte	0x1
	.byte	0x47
	.byte	0xd
	.long	0xccd
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.byte	0
	.uleb128 0x2f
	.long	.LASF120
	.byte	0x1
	.byte	0x42
	.byte	0xd
	.long	0xce4
	.uleb128 0x20
	.long	0x3bc
	.uleb128 0x20
	.long	0x41d
	.byte	0
	.uleb128 0x30
	.long	.LASF121
	.byte	0x10
	.byte	0x43
	.byte	0xd
	.long	0xd05
	.uleb128 0x20
	.long	0x356
	.uleb128 0x20
	.long	0x356
	.uleb128 0x20
	.long	0x6b
	.uleb128 0x20
	.long	0x356
	.byte	0
	.uleb128 0x2d
	.long	.LASF122
	.byte	0x11
	.value	0x1d0
	.byte	0x15
	.long	0xd2c
	.uleb128 0x20
	.long	0x48
	.uleb128 0x20
	.long	0x48
	.uleb128 0x20
	.long	0x48
	.uleb128 0x20
	.long	0x356
	.uleb128 0x20
	.long	0x6b
	.byte	0
	.uleb128 0x31
	.long	.LASF123
	.byte	0x1
	.value	0x359
	.byte	0x5
	.long	0x48
	.quad	.LFB169
	.quad	.LFE169-.LFB169
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x31
	.long	.LASF124
	.byte	0x1
	.value	0x352
	.byte	0x11
	.long	0x320
	.quad	.LFB168
	.quad	.LFE168-.LFB168
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x31
	.long	.LASF125
	.byte	0x1
	.value	0x34b
	.byte	0x11
	.long	0x320
	.quad	.LFB167
	.quad	.LFE167-.LFB167
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x32
	.long	.LASF136
	.byte	0x1
	.value	0x2f5
	.byte	0xc
	.long	0x48
	.quad	.LFB166
	.quad	.LFE166-.LFB166
	.uleb128 0x1
	.byte	0x9c
	.long	0xea3
	.uleb128 0x33
	.string	"ctx"
	.byte	0x1
	.value	0x2f5
	.byte	0x3d
	.long	0x721
	.uleb128 0x3
	.byte	0x91
	.sleb128 -344
	.uleb128 0x33
	.string	"out"
	.byte	0x1
	.value	0x2f5
	.byte	0x4b
	.long	0x41d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -352
	.uleb128 0x34
	.long	.LASF126
	.byte	0x1
	.value	0x2f6
	.byte	0x38
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -360
	.uleb128 0x34
	.long	.LASF29
	.byte	0x1
	.value	0x2f6
	.byte	0x46
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -368
	.uleb128 0x33
	.string	"in"
	.byte	0x1
	.value	0x2f7
	.byte	0x38
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -376
	.uleb128 0x34
	.long	.LASF127
	.byte	0x1
	.value	0x2f7
	.byte	0x43
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -384
	.uleb128 0x34
	.long	.LASF128
	.byte	0x1
	.value	0x2f8
	.byte	0x38
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x34
	.long	.LASF129
	.byte	0x1
	.value	0x2f9
	.byte	0x30
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 8
	.uleb128 0x33
	.string	"ad"
	.byte	0x1
	.value	0x2f9
	.byte	0x4b
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 16
	.uleb128 0x34
	.long	.LASF130
	.byte	0x1
	.value	0x2fa
	.byte	0x30
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 24
	.uleb128 0x29
	.long	.LASF131
	.byte	0x1
	.value	0x2fb
	.byte	0x12
	.long	0xf8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.long	.LASF132
	.byte	0x1
	.value	0x301
	.byte	0x12
	.long	0xf8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x29
	.long	.LASF133
	.byte	0x1
	.value	0x30d
	.byte	0x26
	.long	0xea3
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.long	.LASF134
	.byte	0x1
	.value	0x310
	.byte	0x1e
	.long	0x93c
	.uleb128 0x3
	.byte	0x91
	.sleb128 -320
	.uleb128 0x29
	.long	.LASF135
	.byte	0x1
	.value	0x315
	.byte	0xb
	.long	0x346
	.uleb128 0x3
	.byte	0x91
	.sleb128 -336
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x913
	.uleb128 0x32
	.long	.LASF137
	.byte	0x1
	.value	0x2cc
	.byte	0xc
	.long	0x48
	.quad	.LFB165
	.quad	.LFE165-.LFB165
	.uleb128 0x1
	.byte	0x9c
	.long	0xff3
	.uleb128 0x33
	.string	"ctx"
	.byte	0x1
	.value	0x2cd
	.byte	0x19
	.long	0x721
	.uleb128 0x3
	.byte	0x91
	.sleb128 -344
	.uleb128 0x33
	.string	"out"
	.byte	0x1
	.value	0x2cd
	.byte	0x27
	.long	0x41d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -352
	.uleb128 0x34
	.long	.LASF138
	.byte	0x1
	.value	0x2cd
	.byte	0x35
	.long	0x41d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -360
	.uleb128 0x34
	.long	.LASF139
	.byte	0x1
	.value	0x2ce
	.byte	0xd
	.long	0x727
	.uleb128 0x3
	.byte	0x91
	.sleb128 -368
	.uleb128 0x34
	.long	.LASF140
	.byte	0x1
	.value	0x2ce
	.byte	0x21
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -376
	.uleb128 0x34
	.long	.LASF126
	.byte	0x1
	.value	0x2ce
	.byte	0x41
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -384
	.uleb128 0x34
	.long	.LASF29
	.byte	0x1
	.value	0x2cf
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x33
	.string	"in"
	.byte	0x1
	.value	0x2cf
	.byte	0x26
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 8
	.uleb128 0x34
	.long	.LASF127
	.byte	0x1
	.value	0x2cf
	.byte	0x31
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 16
	.uleb128 0x34
	.long	.LASF141
	.byte	0x1
	.value	0x2cf
	.byte	0x48
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 24
	.uleb128 0x34
	.long	.LASF142
	.byte	0x1
	.value	0x2d0
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 32
	.uleb128 0x33
	.string	"ad"
	.byte	0x1
	.value	0x2d0
	.byte	0x29
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 40
	.uleb128 0x34
	.long	.LASF130
	.byte	0x1
	.value	0x2d0
	.byte	0x34
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 48
	.uleb128 0x29
	.long	.LASF133
	.byte	0x1
	.value	0x2d1
	.byte	0x26
	.long	0xea3
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.long	.LASF132
	.byte	0x1
	.value	0x2d3
	.byte	0x12
	.long	0xf8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x29
	.long	.LASF131
	.byte	0x1
	.value	0x2d4
	.byte	0x12
	.long	0xf8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.long	.LASF134
	.byte	0x1
	.value	0x2e6
	.byte	0x1e
	.long	0x93c
	.uleb128 0x3
	.byte	0x91
	.sleb128 -320
	.uleb128 0x35
	.string	"tag"
	.byte	0x1
	.value	0x2e9
	.byte	0xb
	.long	0x346
	.uleb128 0x3
	.byte	0x91
	.sleb128 -336
	.byte	0
	.uleb128 0x36
	.long	.LASF148
	.byte	0x1
	.value	0x2ac
	.byte	0xd
	.quad	.LFB164
	.quad	.LFE164-.LFB164
	.uleb128 0x1
	.byte	0x9c
	.long	0x10cb
	.uleb128 0x34
	.long	.LASF133
	.byte	0x1
	.value	0x2ac
	.byte	0x3d
	.long	0xea3
	.uleb128 0x3
	.byte	0x91
	.sleb128 -136
	.uleb128 0x34
	.long	.LASF143
	.byte	0x1
	.value	0x2ad
	.byte	0x36
	.long	0x10cb
	.uleb128 0x3
	.byte	0x91
	.sleb128 -144
	.uleb128 0x34
	.long	.LASF126
	.byte	0x1
	.value	0x2ae
	.byte	0x28
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -152
	.uleb128 0x35
	.string	"key"
	.byte	0x1
	.value	0x2af
	.byte	0x18
	.long	0x4fc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x29
	.long	.LASF144
	.byte	0x1
	.value	0x2b0
	.byte	0xb
	.long	0x10d1
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x29
	.long	.LASF145
	.byte	0x1
	.value	0x2b1
	.byte	0x10
	.long	0x3c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.long	.LASF146
	.byte	0x1
	.value	0x2b3
	.byte	0xb
	.long	0x346
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x37
	.quad	.LBB9
	.quad	.LBE9-.LBB9
	.uleb128 0x35
	.string	"i"
	.byte	0x1
	.value	0x2b7
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x37
	.quad	.LBB10
	.quad	.LBE10-.LBB10
	.uleb128 0x29
	.long	.LASF147
	.byte	0x1
	.value	0x2ba
	.byte	0xd
	.long	0x346
	.uleb128 0x3
	.byte	0x91
	.sleb128 -128
	.byte	0
	.byte	0
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x93c
	.uleb128 0x16
	.long	0xbe
	.long	0x10e1
	.uleb128 0x1a
	.long	0x41
	.byte	0x2f
	.byte	0
	.uleb128 0x36
	.long	.LASF149
	.byte	0x1
	.value	0x27c
	.byte	0xd
	.quad	.LFB163
	.quad	.LFE163-.LFB163
	.uleb128 0x1
	.byte	0x9c
	.long	0x11c8
	.uleb128 0x34
	.long	.LASF138
	.byte	0x1
	.value	0x27d
	.byte	0xd
	.long	0x41d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -360
	.uleb128 0x33
	.string	"in"
	.byte	0x1
	.value	0x27d
	.byte	0x29
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -368
	.uleb128 0x34
	.long	.LASF127
	.byte	0x1
	.value	0x27d
	.byte	0x34
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -376
	.uleb128 0x33
	.string	"ad"
	.byte	0x1
	.value	0x27d
	.byte	0x4b
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -384
	.uleb128 0x34
	.long	.LASF130
	.byte	0x1
	.value	0x27e
	.byte	0xc
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -392
	.uleb128 0x34
	.long	.LASF91
	.byte	0x1
	.value	0x27e
	.byte	0x22
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -400
	.uleb128 0x34
	.long	.LASF126
	.byte	0x1
	.value	0x27f
	.byte	0x13
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x29
	.long	.LASF81
	.byte	0x1
	.value	0x280
	.byte	0x16
	.long	0x632
	.uleb128 0x3
	.byte	0x91
	.sleb128 -320
	.uleb128 0x29
	.long	.LASF150
	.byte	0x1
	.value	0x285
	.byte	0xb
	.long	0x346
	.uleb128 0x3
	.byte	0x91
	.sleb128 -336
	.uleb128 0x29
	.long	.LASF151
	.byte	0x1
	.value	0x293
	.byte	0xb
	.long	0x346
	.uleb128 0x3
	.byte	0x91
	.sleb128 -352
	.uleb128 0x37
	.quad	.LBB8
	.quad	.LBE8-.LBB8
	.uleb128 0x35
	.string	"i"
	.byte	0x1
	.value	0x29a
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.byte	0
	.uleb128 0x36
	.long	.LASF152
	.byte	0x1
	.value	0x260
	.byte	0xd
	.quad	.LFB162
	.quad	.LFE162-.LFB162
	.uleb128 0x1
	.byte	0x9c
	.long	0x12d2
	.uleb128 0x33
	.string	"out"
	.byte	0x1
	.value	0x260
	.byte	0x24
	.long	0x41d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x33
	.string	"in"
	.byte	0x1
	.value	0x260
	.byte	0x38
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x34
	.long	.LASF127
	.byte	0x1
	.value	0x260
	.byte	0x43
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x34
	.long	.LASF153
	.byte	0x1
	.value	0x261
	.byte	0x29
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x34
	.long	.LASF93
	.byte	0x1
	.value	0x262
	.byte	0x26
	.long	0x4cf
	.uleb128 0x3
	.byte	0x91
	.sleb128 -120
	.uleb128 0x33
	.string	"key"
	.byte	0x1
	.value	0x262
	.byte	0x40
	.long	0x4f6
	.uleb128 0x3
	.byte	0x91
	.sleb128 -128
	.uleb128 0x29
	.long	.LASF146
	.byte	0x1
	.value	0x263
	.byte	0xb
	.long	0x346
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x37
	.quad	.LBB5
	.quad	.LBE5-.LBB5
	.uleb128 0x29
	.long	.LASF154
	.byte	0x1
	.value	0x268
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x37
	.quad	.LBB6
	.quad	.LBE6-.LBB6
	.uleb128 0x29
	.long	.LASF155
	.byte	0x1
	.value	0x269
	.byte	0xd
	.long	0x346
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x29
	.long	.LASF156
	.byte	0x1
	.value	0x26d
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x37
	.quad	.LBB7
	.quad	.LBE7-.LBB7
	.uleb128 0x35
	.string	"i"
	.byte	0x1
	.value	0x272
	.byte	0x11
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.byte	0
	.byte	0
	.byte	0
	.uleb128 0x38
	.long	.LASF157
	.byte	0x1
	.value	0x257
	.byte	0xd
	.quad	.LFB161
	.quad	.LFE161-.LFB161
	.uleb128 0x1
	.byte	0x9c
	.long	0x1302
	.uleb128 0x33
	.string	"ctx"
	.byte	0x1
	.value	0x257
	.byte	0x34
	.long	0x69f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x32
	.long	.LASF158
	.byte	0x1
	.value	0x23a
	.byte	0xc
	.long	0x48
	.quad	.LFB160
	.quad	.LFE160-.LFB160
	.uleb128 0x1
	.byte	0x9c
	.long	0x1386
	.uleb128 0x33
	.string	"ctx"
	.byte	0x1
	.value	0x23a
	.byte	0x30
	.long	0x69f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x33
	.string	"key"
	.byte	0x1
	.value	0x23a
	.byte	0x44
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x34
	.long	.LASF28
	.byte	0x1
	.value	0x23b
	.byte	0x29
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x34
	.long	.LASF41
	.byte	0x1
	.value	0x23b
	.byte	0x39
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x29
	.long	.LASF159
	.byte	0x1
	.value	0x23c
	.byte	0x10
	.long	0x3c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.long	.LASF133
	.byte	0x1
	.value	0x24b
	.byte	0x20
	.long	0x1386
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x8d6
	.uleb128 0x32
	.long	.LASF160
	.byte	0x1
	.value	0x198
	.byte	0xc
	.long	0x48
	.quad	.LFB159
	.quad	.LFE159-.LFB159
	.uleb128 0x1
	.byte	0x9c
	.long	0x152d
	.uleb128 0x33
	.string	"ctx"
	.byte	0x1
	.value	0x199
	.byte	0x19
	.long	0x721
	.uleb128 0x2
	.byte	0x77
	.sleb128 40
	.uleb128 0x33
	.string	"out"
	.byte	0x1
	.value	0x199
	.byte	0x27
	.long	0x41d
	.uleb128 0x2
	.byte	0x77
	.sleb128 32
	.uleb128 0x34
	.long	.LASF126
	.byte	0x1
	.value	0x199
	.byte	0x3b
	.long	0x3bc
	.uleb128 0x2
	.byte	0x77
	.sleb128 24
	.uleb128 0x34
	.long	.LASF29
	.byte	0x1
	.value	0x19a
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x77
	.sleb128 16
	.uleb128 0x33
	.string	"in"
	.byte	0x1
	.value	0x19a
	.byte	0x26
	.long	0x3bc
	.uleb128 0x2
	.byte	0x77
	.sleb128 8
	.uleb128 0x34
	.long	.LASF127
	.byte	0x1
	.value	0x19a
	.byte	0x31
	.long	0x30
	.uleb128 0x2
	.byte	0x77
	.sleb128 0
	.uleb128 0x34
	.long	.LASF128
	.byte	0x1
	.value	0x19a
	.byte	0x48
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x34
	.long	.LASF129
	.byte	0x1
	.value	0x19b
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 8
	.uleb128 0x33
	.string	"ad"
	.byte	0x1
	.value	0x19b
	.byte	0x27
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 16
	.uleb128 0x34
	.long	.LASF130
	.byte	0x1
	.value	0x19b
	.byte	0x32
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 24
	.uleb128 0x29
	.long	.LASF131
	.byte	0x1
	.value	0x19c
	.byte	0x12
	.long	0xf8
	.uleb128 0x3
	.byte	0x77
	.sleb128 640
	.uleb128 0x29
	.long	.LASF132
	.byte	0x1
	.value	0x1a2
	.byte	0x12
	.long	0xf8
	.uleb128 0x3
	.byte	0x77
	.sleb128 632
	.uleb128 0x29
	.long	.LASF133
	.byte	0x1
	.value	0x1ae
	.byte	0x2a
	.long	0xa7f
	.uleb128 0x3
	.byte	0x77
	.sleb128 624
	.uleb128 0x39
	.long	.LASF162
	.byte	0x1
	.value	0x1b4
	.byte	0x18
	.long	0x152d
	.byte	0x10
	.uleb128 0x3
	.byte	0x77
	.sleb128 592
	.uleb128 0x39
	.long	.LASF163
	.byte	0x1
	.value	0x1b5
	.byte	0x18
	.long	0x153d
	.byte	0x10
	.uleb128 0x3
	.byte	0x77
	.sleb128 560
	.uleb128 0x29
	.long	.LASF164
	.byte	0x1
	.value	0x1b9
	.byte	0x23
	.long	0x844
	.uleb128 0x3
	.byte	0x77
	.sleb128 304
	.uleb128 0x39
	.long	.LASF165
	.byte	0x1
	.value	0x1c1
	.byte	0x17
	.long	0x35c
	.byte	0x10
	.uleb128 0x3
	.byte	0x77
	.sleb128 176
	.uleb128 0x29
	.long	.LASF166
	.byte	0x1
	.value	0x1c4
	.byte	0x10
	.long	0x3c
	.uleb128 0x3
	.byte	0x77
	.sleb128 616
	.uleb128 0x29
	.long	.LASF150
	.byte	0x1
	.value	0x1c8
	.byte	0xb
	.long	0x346
	.uleb128 0x3
	.byte	0x77
	.sleb128 160
	.uleb128 0x39
	.long	.LASF167
	.byte	0x1
	.value	0x1d0
	.byte	0x17
	.long	0x154d
	.byte	0x10
	.uleb128 0x3
	.byte	0x77
	.sleb128 64
	.uleb128 0x29
	.long	.LASF151
	.byte	0x1
	.value	0x1e5
	.byte	0xb
	.long	0x346
	.uleb128 0x2
	.byte	0x77
	.sleb128 48
	.uleb128 0x37
	.quad	.LBB4
	.quad	.LBE4-.LBB4
	.uleb128 0x35
	.string	"i"
	.byte	0x1
	.value	0x1eb
	.byte	0xf
	.long	0x30
	.uleb128 0x3
	.byte	0x77
	.sleb128 648
	.byte	0
	.byte	0
	.uleb128 0x16
	.long	0xec
	.long	0x153d
	.uleb128 0x1a
	.long	0x41
	.byte	0x1
	.byte	0
	.uleb128 0x16
	.long	0xec
	.long	0x154d
	.uleb128 0x1a
	.long	0x41
	.byte	0x3
	.byte	0
	.uleb128 0x16
	.long	0xbe
	.long	0x155d
	.uleb128 0x1a
	.long	0x41
	.byte	0x5f
	.byte	0
	.uleb128 0x32
	.long	.LASF168
	.byte	0x1
	.value	0x152
	.byte	0xc
	.long	0x48
	.quad	.LFB158
	.quad	.LFE158-.LFB158
	.uleb128 0x1
	.byte	0x9c
	.long	0x16cb
	.uleb128 0x33
	.string	"ctx"
	.byte	0x1
	.value	0x153
	.byte	0x19
	.long	0x721
	.uleb128 0x3
	.byte	0x91
	.sleb128 -376
	.uleb128 0x33
	.string	"out"
	.byte	0x1
	.value	0x153
	.byte	0x27
	.long	0x41d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -384
	.uleb128 0x34
	.long	.LASF138
	.byte	0x1
	.value	0x153
	.byte	0x35
	.long	0x41d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -392
	.uleb128 0x34
	.long	.LASF139
	.byte	0x1
	.value	0x154
	.byte	0xd
	.long	0x727
	.uleb128 0x3
	.byte	0x91
	.sleb128 -400
	.uleb128 0x34
	.long	.LASF140
	.byte	0x1
	.value	0x154
	.byte	0x21
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -408
	.uleb128 0x34
	.long	.LASF126
	.byte	0x1
	.value	0x154
	.byte	0x41
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -416
	.uleb128 0x34
	.long	.LASF29
	.byte	0x1
	.value	0x155
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x33
	.string	"in"
	.byte	0x1
	.value	0x155
	.byte	0x26
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 8
	.uleb128 0x34
	.long	.LASF127
	.byte	0x1
	.value	0x155
	.byte	0x31
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 16
	.uleb128 0x34
	.long	.LASF141
	.byte	0x1
	.value	0x155
	.byte	0x48
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 24
	.uleb128 0x34
	.long	.LASF142
	.byte	0x1
	.value	0x156
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 32
	.uleb128 0x33
	.string	"ad"
	.byte	0x1
	.value	0x156
	.byte	0x29
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 40
	.uleb128 0x34
	.long	.LASF130
	.byte	0x1
	.value	0x156
	.byte	0x34
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 48
	.uleb128 0x29
	.long	.LASF133
	.byte	0x1
	.value	0x157
	.byte	0x2a
	.long	0xa7f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x29
	.long	.LASF132
	.byte	0x1
	.value	0x15c
	.byte	0x12
	.long	0xf8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x29
	.long	.LASF131
	.byte	0x1
	.value	0x15d
	.byte	0x12
	.long	0xf8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x39
	.long	.LASF162
	.byte	0x1
	.value	0x16e
	.byte	0x18
	.long	0x152d
	.byte	0x10
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x39
	.long	.LASF163
	.byte	0x1
	.value	0x16f
	.byte	0x18
	.long	0x153d
	.byte	0x10
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x3a
	.string	"tag"
	.byte	0x1
	.value	0x173
	.byte	0x17
	.long	0x346
	.byte	0x10
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x29
	.long	.LASF169
	.byte	0x1
	.value	0x177
	.byte	0x23
	.long	0x844
	.uleb128 0x3
	.byte	0x91
	.sleb128 -368
	.byte	0
	.uleb128 0x36
	.long	.LASF170
	.byte	0x1
	.value	0x13a
	.byte	0xd
	.quad	.LFB157
	.quad	.LFE157-.LFB157
	.uleb128 0x1
	.byte	0x9c
	.long	0x1763
	.uleb128 0x34
	.long	.LASF83
	.byte	0x1
	.value	0x13b
	.byte	0x9
	.long	0x48
	.uleb128 0x3
	.byte	0x91
	.sleb128 -132
	.uleb128 0x34
	.long	.LASF133
	.byte	0x1
	.value	0x13b
	.byte	0x3c
	.long	0xa7f
	.uleb128 0x3
	.byte	0x91
	.sleb128 -144
	.uleb128 0x34
	.long	.LASF171
	.byte	0x1
	.value	0x13c
	.byte	0xe
	.long	0xbc3
	.uleb128 0x3
	.byte	0x91
	.sleb128 -152
	.uleb128 0x34
	.long	.LASF172
	.byte	0x1
	.value	0x13c
	.byte	0x2f
	.long	0xbc3
	.uleb128 0x3
	.byte	0x91
	.sleb128 -160
	.uleb128 0x34
	.long	.LASF126
	.byte	0x1
	.value	0x13d
	.byte	0x13
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -168
	.uleb128 0x39
	.long	.LASF173
	.byte	0x1
	.value	0x13e
	.byte	0x17
	.long	0x346
	.byte	0x10
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x39
	.long	.LASF144
	.byte	0x1
	.value	0x141
	.byte	0x18
	.long	0x1763
	.byte	0x10
	.uleb128 0x3
	.byte	0x91
	.sleb128 -128
	.byte	0
	.uleb128 0x16
	.long	0xec
	.long	0x1773
	.uleb128 0x1a
	.long	0x41
	.byte	0xb
	.byte	0
	.uleb128 0x36
	.long	.LASF174
	.byte	0x1
	.value	0x120
	.byte	0xd
	.quad	.LFB156
	.quad	.LFE156-.LFB156
	.uleb128 0x1
	.byte	0x9c
	.long	0x186a
	.uleb128 0x34
	.long	.LASF83
	.byte	0x1
	.value	0x121
	.byte	0x9
	.long	0x48
	.uleb128 0x3
	.byte	0x91
	.sleb128 -84
	.uleb128 0x33
	.string	"out"
	.byte	0x1
	.value	0x121
	.byte	0x1e
	.long	0x41d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x33
	.string	"in"
	.byte	0x1
	.value	0x121
	.byte	0x32
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x34
	.long	.LASF127
	.byte	0x1
	.value	0x121
	.byte	0x3d
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x33
	.string	"tag"
	.byte	0x1
	.value	0x122
	.byte	0x13
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -120
	.uleb128 0x34
	.long	.LASF169
	.byte	0x1
	.value	0x123
	.byte	0x2c
	.long	0xa7f
	.uleb128 0x3
	.byte	0x91
	.sleb128 -128
	.uleb128 0x39
	.long	.LASF146
	.byte	0x1
	.value	0x124
	.byte	0x17
	.long	0x346
	.byte	0x10
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x29
	.long	.LASF175
	.byte	0x1
	.value	0x12f
	.byte	0x10
	.long	0x3c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x29
	.long	.LASF176
	.byte	0x1
	.value	0x130
	.byte	0x10
	.long	0x3c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x29
	.long	.LASF177
	.byte	0x1
	.value	0x131
	.byte	0xc
	.long	0x41d
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x29
	.long	.LASF178
	.byte	0x1
	.value	0x132
	.byte	0x12
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x37
	.quad	.LBB3
	.quad	.LBE3-.LBB3
	.uleb128 0x35
	.string	"i"
	.byte	0x1
	.value	0x133
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.byte	0
	.uleb128 0x3b
	.long	.LASF179
	.byte	0x1
	.byte	0xe8
	.byte	0xd
	.quad	.LFB155
	.quad	.LFE155-.LFB155
	.uleb128 0x1
	.byte	0x9c
	.long	0x1975
	.uleb128 0x3c
	.long	.LASF138
	.byte	0x1
	.byte	0xe8
	.byte	0x29
	.long	0x41d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -216
	.uleb128 0x3d
	.string	"in"
	.byte	0x1
	.byte	0xe8
	.byte	0x45
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -224
	.uleb128 0x3c
	.long	.LASF127
	.byte	0x1
	.byte	0xe9
	.byte	0x28
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -232
	.uleb128 0x3d
	.string	"ad"
	.byte	0x1
	.byte	0xe9
	.byte	0x3f
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -240
	.uleb128 0x3c
	.long	.LASF130
	.byte	0x1
	.byte	0xe9
	.byte	0x4a
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -248
	.uleb128 0x3c
	.long	.LASF91
	.byte	0x1
	.byte	0xea
	.byte	0x2f
	.long	0x3bc
	.uleb128 0x3
	.byte	0x91
	.sleb128 -256
	.uleb128 0x3c
	.long	.LASF126
	.byte	0x1
	.byte	0xeb
	.byte	0x2f
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x3e
	.long	.LASF166
	.byte	0x1
	.byte	0xed
	.byte	0x10
	.long	0x3c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x3e
	.long	.LASF180
	.byte	0x1
	.byte	0xee
	.byte	0x10
	.long	0x3c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x3e
	.long	.LASF181
	.byte	0x1
	.byte	0xef
	.byte	0x7
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x3f
	.long	.LASF167
	.byte	0x1
	.byte	0xf0
	.byte	0x17
	.long	0x35c
	.byte	0x10
	.uleb128 0x3
	.byte	0x91
	.sleb128 -176
	.uleb128 0x3e
	.long	.LASF150
	.byte	0x1
	.byte	0xfd
	.byte	0xb
	.long	0x346
	.uleb128 0x3
	.byte	0x91
	.sleb128 -192
	.uleb128 0x29
	.long	.LASF151
	.byte	0x1
	.value	0x110
	.byte	0xb
	.long	0x346
	.uleb128 0x3
	.byte	0x91
	.sleb128 -208
	.uleb128 0x37
	.quad	.LBB2
	.quad	.LBE2-.LBB2
	.uleb128 0x35
	.string	"i"
	.byte	0x1
	.value	0x115
	.byte	0xf
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.byte	0
	.uleb128 0x40
	.long	.LASF182
	.byte	0x1
	.byte	0x71
	.byte	0xd
	.quad	.LFB154
	.quad	.LFE154-.LFB154
	.uleb128 0x1
	.byte	0x9c
	.long	0x19a3
	.uleb128 0x3d
	.string	"ctx"
	.byte	0x1
	.byte	0x71
	.byte	0x38
	.long	0x69f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x41
	.long	.LASF183
	.byte	0x1
	.byte	0x4a
	.byte	0xc
	.long	0x48
	.quad	.LFB153
	.quad	.LFE153-.LFB153
	.uleb128 0x1
	.byte	0x9c
	.long	0x1a33
	.uleb128 0x3d
	.string	"ctx"
	.byte	0x1
	.byte	0x4a
	.byte	0x34
	.long	0x69f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x3d
	.string	"key"
	.byte	0x1
	.byte	0x4a
	.byte	0x48
	.long	0x3bc
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x3c
	.long	.LASF28
	.byte	0x1
	.byte	0x4b
	.byte	0x2d
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x3c
	.long	.LASF41
	.byte	0x1
	.byte	0x4b
	.byte	0x3d
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x3e
	.long	.LASF159
	.byte	0x1
	.byte	0x4c
	.byte	0x10
	.long	0x3c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x3e
	.long	.LASF133
	.byte	0x1
	.byte	0x5d
	.byte	0x24
	.long	0x1a33
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x42
	.long	.LASF199
	.long	0x1a49
	.uleb128 0x9
	.byte	0x3
	.quad	__PRETTY_FUNCTION__.0
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x844
	.uleb128 0x16
	.long	0xb9
	.long	0x1a49
	.uleb128 0x1a
	.long	0x41
	.byte	0x19
	.byte	0
	.uleb128 0x4
	.long	0x1a39
	.uleb128 0x43
	.long	.LASF184
	.byte	0x1
	.byte	0x35
	.byte	0x29
	.long	0x1a33
	.quad	.LFB152
	.quad	.LFE152-.LFB152
	.uleb128 0x1
	.byte	0x9c
	.long	0x1a8f
	.uleb128 0x3d
	.string	"ctx"
	.byte	0x1
	.byte	0x36
	.byte	0x19
	.long	0x721
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x3e
	.long	.LASF185
	.byte	0x1
	.byte	0x39
	.byte	0x13
	.long	0x109
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x44
	.long	.LASF186
	.byte	0x3
	.byte	0x59
	.byte	0x14
	.long	0x48
	.quad	.LFB136
	.quad	.LFE136-.LFB136
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x44
	.long	.LASF187
	.byte	0x3
	.byte	0x52
	.byte	0x14
	.long	0x48
	.quad	.LFB135
	.quad	.LFE135-.LFB135
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x45
	.long	.LASF188
	.byte	0x3
	.byte	0x30
	.byte	0x20
	.long	0x1ae9
	.quad	.LFB128
	.quad	.LFE128-.LFB128
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x7
	.byte	0x8
	.long	0xe7
	.uleb128 0x36
	.long	.LASF189
	.byte	0x2
	.value	0x42c
	.byte	0x14
	.quad	.LFB106
	.quad	.LFE106-.LFB106
	.uleb128 0x1
	.byte	0x9c
	.long	0x1b2d
	.uleb128 0x33
	.string	"out"
	.byte	0x2
	.value	0x42c
	.byte	0x2e
	.long	0xb0
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x33
	.string	"v"
	.byte	0x2
	.value	0x42c
	.byte	0x3c
	.long	0xec
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x36
	.long	.LASF190
	.byte	0x2
	.value	0x407
	.byte	0x14
	.quad	.LFB102
	.quad	.LFE102-.LFB102
	.uleb128 0x1
	.byte	0x9c
	.long	0x1b6b
	.uleb128 0x33
	.string	"out"
	.byte	0x2
	.value	0x407
	.byte	0x2e
	.long	0xb0
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x33
	.string	"v"
	.byte	0x2
	.value	0x407
	.byte	0x3c
	.long	0xdb
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x32
	.long	.LASF191
	.byte	0x2
	.value	0x3fd
	.byte	0x18
	.long	0xdb
	.quad	.LFB101
	.quad	.LFE101-.LFB101
	.uleb128 0x1
	.byte	0x9c
	.long	0x1bac
	.uleb128 0x33
	.string	"in"
	.byte	0x2
	.value	0x3fd
	.byte	0x37
	.long	0x115
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x35
	.string	"v"
	.byte	0x2
	.value	0x3fe
	.byte	0xc
	.long	0xdb
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.uleb128 0x32
	.long	.LASF192
	.byte	0x2
	.value	0x3cc
	.byte	0x15
	.long	0xb0
	.quad	.LFB96
	.quad	.LFE96-.LFB96
	.uleb128 0x1
	.byte	0x9c
	.long	0x1bfc
	.uleb128 0x33
	.string	"dst"
	.byte	0x2
	.value	0x3cc
	.byte	0x2a
	.long	0xb0
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x33
	.string	"c"
	.byte	0x2
	.value	0x3cc
	.byte	0x33
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.uleb128 0x33
	.string	"n"
	.byte	0x2
	.value	0x3cc
	.byte	0x3d
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x46
	.long	.LASF193
	.byte	0x2
	.value	0x3bc
	.byte	0x15
	.long	0xb0
	.quad	.LFB94
	.quad	.LFE94-.LFB94
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x33
	.string	"dst"
	.byte	0x2
	.value	0x3bc
	.byte	0x2a
	.long	0xb0
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x33
	.string	"src"
	.byte	0x2
	.value	0x3bc
	.byte	0x3b
	.long	0x115
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x33
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
	.uleb128 0xe
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
	.uleb128 0x11
	.uleb128 0x13
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
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
	.uleb128 0x12
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
	.uleb128 0x13
	.uleb128 0x17
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
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
	.uleb128 0x14
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
	.uleb128 0x15
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
	.byte	0
	.byte	0
	.uleb128 0x16
	.uleb128 0x1
	.byte	0x1
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x17
	.uleb128 0x21
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2f
	.uleb128 0x5
	.byte	0
	.byte	0
	.uleb128 0x18
	.uleb128 0x4
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x3e
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0x49
	.uleb128 0x13
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
	.uleb128 0x19
	.uleb128 0x28
	.byte	0
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0x1c
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x1a
	.uleb128 0x21
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.uleb128 0x2f
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x1b
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
	.uleb128 0x1c
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
	.uleb128 0x1d
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
	.uleb128 0x1e
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
	.uleb128 0x1f
	.uleb128 0x15
	.byte	0x1
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x20
	.uleb128 0x5
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x21
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
	.uleb128 0x22
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
	.uleb128 0xb
	.uleb128 0xb
	.uleb128 0xd
	.uleb128 0xb
	.uleb128 0xc
	.uleb128 0xb
	.uleb128 0x38
	.uleb128 0x5
	.byte	0
	.byte	0
	.uleb128 0x23
	.uleb128 0x13
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0xb
	.uleb128 0x5
	.uleb128 0x88
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
	.uleb128 0x24
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
	.uleb128 0x88
	.uleb128 0xb
	.uleb128 0x38
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x25
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
	.uleb128 0x5
	.byte	0
	.byte	0
	.uleb128 0x26
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
	.uleb128 0x27
	.uleb128 0x13
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0xb
	.uleb128 0x5
	.uleb128 0x88
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
	.uleb128 0x28
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
	.uleb128 0x88
	.uleb128 0xb
	.uleb128 0x38
	.uleb128 0xb
	.byte	0
	.byte	0
	.uleb128 0x29
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
	.uleb128 0x2a
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
	.byte	0
	.byte	0
	.uleb128 0x2b
	.uleb128 0x13
	.byte	0x1
	.uleb128 0x3
	.uleb128 0xe
	.uleb128 0xb
	.uleb128 0x5
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
	.uleb128 0x2c
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
	.uleb128 0x5
	.byte	0
	.byte	0
	.uleb128 0x2d
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
	.uleb128 0x2e
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
	.uleb128 0x3c
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x30
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
	.uleb128 0x31
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
	.uleb128 0x2116
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x33
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
	.uleb128 0x34
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
	.uleb128 0x35
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
	.uleb128 0x36
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
	.uleb128 0x37
	.uleb128 0xb
	.byte	0x1
	.uleb128 0x11
	.uleb128 0x1
	.uleb128 0x12
	.uleb128 0x7
	.byte	0
	.byte	0
	.uleb128 0x38
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
	.uleb128 0x39
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
	.uleb128 0x88
	.uleb128 0xb
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x3a
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
	.uleb128 0x88
	.uleb128 0xb
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x3b
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
	.uleb128 0x3c
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
	.uleb128 0x3d
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
	.uleb128 0x3e
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
	.uleb128 0x3f
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
	.uleb128 0x88
	.uleb128 0xb
	.uleb128 0x2
	.uleb128 0x18
	.byte	0
	.byte	0
	.uleb128 0x40
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
	.uleb128 0x2117
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x41
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
	.uleb128 0x42
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
	.uleb128 0x43
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
	.uleb128 0x44
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
	.uleb128 0x45
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
	.uleb128 0x46
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
	.long	0x1bc
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
	.quad	.LFB101
	.quad	.LFE101-.LFB101
	.quad	.LFB102
	.quad	.LFE102-.LFB102
	.quad	.LFB106
	.quad	.LFE106-.LFB106
	.quad	.LFB128
	.quad	.LFE128-.LFB128
	.quad	.LFB135
	.quad	.LFE135-.LFB135
	.quad	.LFB136
	.quad	.LFE136-.LFB136
	.quad	.LFB152
	.quad	.LFE152-.LFB152
	.quad	.LFB153
	.quad	.LFE153-.LFB153
	.quad	.LFB154
	.quad	.LFE154-.LFB154
	.quad	.LFB155
	.quad	.LFE155-.LFB155
	.quad	.LFB156
	.quad	.LFE156-.LFB156
	.quad	.LFB157
	.quad	.LFE157-.LFB157
	.quad	.LFB158
	.quad	.LFE158-.LFB158
	.quad	.LFB159
	.quad	.LFE159-.LFB159
	.quad	.LFB160
	.quad	.LFE160-.LFB160
	.quad	.LFB161
	.quad	.LFE161-.LFB161
	.quad	.LFB162
	.quad	.LFE162-.LFB162
	.quad	.LFB163
	.quad	.LFE163-.LFB163
	.quad	.LFB164
	.quad	.LFE164-.LFB164
	.quad	.LFB165
	.quad	.LFE165-.LFB165
	.quad	.LFB166
	.quad	.LFE166-.LFB166
	.quad	.LFB167
	.quad	.LFE167-.LFB167
	.quad	.LFB168
	.quad	.LFE168-.LFB168
	.quad	.LFB169
	.quad	.LFE169-.LFB169
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LFB94
	.quad	.LFE94
	.quad	.LFB96
	.quad	.LFE96
	.quad	.LFB101
	.quad	.LFE101
	.quad	.LFB102
	.quad	.LFE102
	.quad	.LFB106
	.quad	.LFE106
	.quad	.LFB128
	.quad	.LFE128
	.quad	.LFB135
	.quad	.LFE135
	.quad	.LFB136
	.quad	.LFE136
	.quad	.LFB152
	.quad	.LFE152
	.quad	.LFB153
	.quad	.LFE153
	.quad	.LFB154
	.quad	.LFE154
	.quad	.LFB155
	.quad	.LFE155
	.quad	.LFB156
	.quad	.LFE156
	.quad	.LFB157
	.quad	.LFE157
	.quad	.LFB158
	.quad	.LFE158
	.quad	.LFB159
	.quad	.LFE159
	.quad	.LFB160
	.quad	.LFE160
	.quad	.LFB161
	.quad	.LFE161
	.quad	.LFB162
	.quad	.LFE162
	.quad	.LFB163
	.quad	.LFE163
	.quad	.LFB164
	.quad	.LFE164
	.quad	.LFB165
	.quad	.LFE165
	.quad	.LFB166
	.quad	.LFE166
	.quad	.LFB167
	.quad	.LFE167
	.quad	.LFB168
	.quad	.LFE168
	.quad	.LFB169
	.quad	.LFE169
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF130:
	.string	"ad_len"
.LASF180:
	.string	"in_blocks"
.LASF99:
	.string	"aws_lc_0_38_0_aes_ctr_set_key"
.LASF55:
	.string	"rd_key"
.LASF172:
	.string	"out_record_enc_key"
.LASF8:
	.string	"size_t"
.LASF19:
	.string	"uintptr_t"
.LASF54:
	.string	"aes_key_st"
.LASF83:
	.string	"is_128_bit"
.LASF61:
	.string	"cbb_child_st"
.LASF18:
	.string	"uint64_t"
.LASF152:
	.string	"gcm_siv_crypt"
.LASF106:
	.string	"aws_lc_0_38_0_aes256gcmsiv_aes_ks_enc_x1"
.LASF9:
	.string	"__uint8_t"
.LASF88:
	.string	"kgk_block"
.LASF64:
	.string	"pending_len_len"
.LASF34:
	.string	"init"
.LASF120:
	.string	"aws_lc_0_38_0_aes128gcmsiv_aes_ks"
.LASF186:
	.string	"CRYPTO_is_AVX_capable"
.LASF145:
	.string	"blocks_needed"
.LASF195:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aesgcmsiv.c"
.LASF49:
	.string	"opaque"
.LASF22:
	.string	"is_child"
.LASF41:
	.string	"tag_len"
.LASF187:
	.string	"CRYPTO_is_AESNI_capable"
.LASF135:
	.string	"expected_tag"
.LASF193:
	.string	"OPENSSL_memcpy"
.LASF113:
	.string	"aws_lc_0_38_0_aes128gcmsiv_ecb_enc_block"
.LASF98:
	.string	"aws_lc_0_38_0_CRYPTO_POLYVAL_init"
.LASF31:
	.string	"max_tag_len"
.LASF95:
	.string	"aead_aes_256_gcm_siv"
.LASF164:
	.string	"expanded_key"
.LASF147:
	.string	"ciphertext"
.LASF7:
	.string	"signed char"
.LASF157:
	.string	"aead_aes_gcm_siv_cleanup"
.LASF29:
	.string	"nonce_len"
.LASF199:
	.string	"__PRETTY_FUNCTION__"
.LASF170:
	.string	"aead_aes_gcm_siv_kdf"
.LASF59:
	.string	"can_resize"
.LASF103:
	.string	"aws_lc_0_38_0_aesgcmsiv_htable6_init"
.LASF21:
	.string	"child"
.LASF198:
	.string	"evp_aead_direction_t"
.LASF73:
	.string	"ghash_func"
.LASF74:
	.string	"gcm128_key_st"
.LASF47:
	.string	"state"
.LASF0:
	.string	"long int"
.LASF102:
	.string	"aws_lc_0_38_0_aes128gcmsiv_dec"
.LASF190:
	.string	"CRYPTO_store_u32_le"
.LASF38:
	.string	"seal_scatter"
.LASF45:
	.string	"evp_aead_ctx_st"
.LASF84:
	.string	"aead_aes_128_gcm_siv_asm"
.LASF93:
	.string	"enc_block"
.LASF16:
	.string	"uint16_t"
.LASF53:
	.string	"double"
.LASF52:
	.string	"evp_aead_seal"
.LASF173:
	.string	"padded_nonce"
.LASF56:
	.string	"rounds"
.LASF28:
	.string	"key_len"
.LASF96:
	.string	"aws_lc_0_38_0_CRYPTO_POLYVAL_finish"
.LASF12:
	.string	"__uint32_t"
.LASF183:
	.string	"aead_aes_gcm_siv_asm_init"
.LASF2:
	.string	"long long int"
.LASF131:
	.string	"ad_len_64"
.LASF86:
	.string	"align"
.LASF133:
	.string	"gcm_siv_ctx"
.LASF104:
	.string	"aws_lc_0_38_0_aes256gcmsiv_enc_msg_x8"
.LASF142:
	.string	"extra_in_len"
.LASF94:
	.string	"aead_aes_128_gcm_siv"
.LASF78:
	.string	"block"
.LASF6:
	.string	"unsigned int"
.LASF48:
	.string	"state_offset"
.LASF66:
	.string	"__int128"
.LASF101:
	.string	"aws_lc_0_38_0_aes256gcmsiv_dec"
.LASF35:
	.string	"init_with_direction"
.LASF1:
	.string	"long unsigned int"
.LASF191:
	.string	"CRYPTO_load_u32_le"
.LASF42:
	.string	"serialize_state"
.LASF174:
	.string	"aead_aes_gcm_siv_asm_crypt_last_block"
.LASF116:
	.string	"aws_lc_0_38_0_aesgcmsiv_polyval_horner"
.LASF25:
	.string	"data"
.LASF79:
	.string	"use_hw_gcm_crypt"
.LASF136:
	.string	"aead_aes_gcm_siv_open_gather"
.LASF109:
	.string	"aws_lc_0_38_0_aes128gcmsiv_aes_ks_enc_x1"
.LASF23:
	.string	"cbb_st"
.LASF160:
	.string	"aead_aes_gcm_siv_asm_open_gather"
.LASF163:
	.string	"record_enc_key"
.LASF197:
	.string	"evp_aead_ctx_st_state"
.LASF179:
	.string	"gcm_siv_asm_polyval"
.LASF181:
	.string	"htable_init"
.LASF117:
	.string	"aws_lc_0_38_0_aesgcmsiv_htable_polyval"
.LASF20:
	.string	"long long unsigned int"
.LASF129:
	.string	"in_tag_len"
.LASF108:
	.string	"aws_lc_0_38_0_aes128gcmsiv_enc_msg_x4"
.LASF90:
	.string	"gcm_siv_record_keys"
.LASF151:
	.string	"length_block"
.LASF107:
	.string	"aws_lc_0_38_0_aes128gcmsiv_enc_msg_x8"
.LASF76:
	.string	"gmult"
.LASF62:
	.string	"base"
.LASF165:
	.string	"calculated_tag"
.LASF46:
	.string	"aead"
.LASF27:
	.string	"evp_aead_st"
.LASF37:
	.string	"open"
.LASF166:
	.string	"ad_blocks"
.LASF148:
	.string	"gcm_siv_keys"
.LASF127:
	.string	"in_len"
.LASF77:
	.string	"ghash"
.LASF30:
	.string	"overhead"
.LASF128:
	.string	"in_tag"
.LASF177:
	.string	"last_bytes_out"
.LASF156:
	.string	"todo"
.LASF92:
	.string	"enc_key"
.LASF26:
	.string	"EVP_AEAD"
.LASF36:
	.string	"cleanup"
.LASF69:
	.string	"block128_f"
.LASF81:
	.string	"polyval_ctx"
.LASF39:
	.string	"open_gather"
.LASF144:
	.string	"key_material"
.LASF134:
	.string	"keys"
.LASF13:
	.string	"__uint64_t"
.LASF65:
	.string	"pending_is_asn1"
.LASF123:
	.string	"aws_lc_0_38_0_x86_64_assembly_implementation_FOR_TESTING"
.LASF153:
	.string	"initial_counter"
.LASF122:
	.string	"aws_lc_0_38_0_ERR_put_error"
.LASF50:
	.string	"alignment"
.LASF97:
	.string	"aws_lc_0_38_0_CRYPTO_POLYVAL_update_blocks"
.LASF121:
	.string	"__assert_fail"
.LASF185:
	.string	"actual_offset"
.LASF155:
	.string	"keystream"
.LASF67:
	.string	"__int128 unsigned"
.LASF68:
	.string	"_Bool"
.LASF158:
	.string	"aead_aes_gcm_siv_init"
.LASF196:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF182:
	.string	"aead_aes_gcm_siv_asm_cleanup"
.LASF24:
	.string	"cbs_st"
.LASF10:
	.string	"short int"
.LASF167:
	.string	"htable"
.LASF194:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF60:
	.string	"error"
.LASF82:
	.string	"aead_aes_gcm_siv_asm_ctx"
.LASF125:
	.string	"aws_lc_0_38_0_EVP_aead_aes_128_gcm_siv"
.LASF162:
	.string	"record_auth_key"
.LASF126:
	.string	"nonce"
.LASF71:
	.string	"u128"
.LASF89:
	.string	"is_256"
.LASF17:
	.string	"uint32_t"
.LASF161:
	.string	"aws_lc_0_38_0_OPENSSL_ia32cap_P"
.LASF149:
	.string	"gcm_siv_polyval"
.LASF87:
	.string	"aead_aes_gcm_siv_ctx"
.LASF75:
	.string	"Htable"
.LASF3:
	.string	"long double"
.LASF112:
	.string	"aws_lc_0_38_0_aes256gcmsiv_ecb_enc_block"
.LASF14:
	.string	"char"
.LASF192:
	.string	"OPENSSL_memset"
.LASF111:
	.string	"aws_lc_0_38_0_aes128gcmsiv_kdf"
.LASF138:
	.string	"out_tag"
.LASF11:
	.string	"__uint16_t"
.LASF132:
	.string	"in_len_64"
.LASF43:
	.string	"deserialize_state"
.LASF85:
	.string	"aead_aes_256_gcm_siv_asm"
.LASF159:
	.string	"key_bits"
.LASF150:
	.string	"scratch"
.LASF143:
	.string	"out_keys"
.LASF91:
	.string	"auth_key"
.LASF146:
	.string	"counter"
.LASF63:
	.string	"offset"
.LASF114:
	.string	"memcpy"
.LASF44:
	.string	"EVP_AEAD_CTX"
.LASF5:
	.string	"short unsigned int"
.LASF4:
	.string	"unsigned char"
.LASF175:
	.string	"last_bytes_offset"
.LASF32:
	.string	"aead_id"
.LASF188:
	.string	"OPENSSL_ia32cap_get"
.LASF184:
	.string	"asm_ctx_from_ctx"
.LASF80:
	.string	"GCM128_KEY"
.LASF124:
	.string	"aws_lc_0_38_0_EVP_aead_aes_256_gcm_siv"
.LASF189:
	.string	"CRYPTO_store_u64_le"
.LASF115:
	.string	"memset"
.LASF141:
	.string	"extra_in"
.LASF178:
	.string	"last_bytes_in"
.LASF118:
	.string	"aws_lc_0_38_0_aesgcmsiv_htable_init"
.LASF15:
	.string	"uint8_t"
.LASF139:
	.string	"out_tag_len"
.LASF70:
	.string	"ctr128_f"
.LASF169:
	.string	"enc_key_expanded"
.LASF40:
	.string	"get_iv"
.LASF105:
	.string	"aws_lc_0_38_0_aes256gcmsiv_enc_msg_x4"
.LASF51:
	.string	"evp_aead_open"
.LASF33:
	.string	"seal_scatter_supports_extra_in"
.LASF140:
	.string	"max_out_tag_len"
.LASF57:
	.string	"AES_KEY"
.LASF171:
	.string	"out_record_auth_key"
.LASF119:
	.string	"aws_lc_0_38_0_aes256gcmsiv_aes_ks"
.LASF110:
	.string	"aws_lc_0_38_0_aes256gcmsiv_kdf"
.LASF58:
	.string	"cbb_buffer_st"
.LASF154:
	.string	"done"
.LASF137:
	.string	"aead_aes_gcm_siv_seal_scatter"
.LASF176:
	.string	"last_bytes_len"
.LASF168:
	.string	"aead_aes_gcm_siv_asm_seal_scatter"
.LASF100:
	.string	"aws_lc_0_38_0_CRYPTO_memcmp"
.LASF72:
	.string	"gmult_func"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
