	.file	"e_aesctrhmac.c"
	.text
.Ltext0:
	.file 1 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aesctrhmac.c"
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
	.section	.text.hmac_init,"ax",@progbits
	.type	hmac_init, @function
hmac_init:
.LFB151:
	.loc 1 46 51
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
	movq	hmac_key_len.1(%rip), %rdx
	movq	-104(%rbp), %rcx
	leaq	-80(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 50 3
	movq	hmac_key_len.1(%rip), %rax
	movl	$64, %edx
	subq	%rax, %rdx
	movq	hmac_key_len.1(%rip), %rax
	leaq	-80(%rbp), %rcx
	addq	%rcx, %rax
	movl	$54, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 53 10
	movl	$0, -4(%rbp)
	.loc 1 53 3
	jmp	.L8
.L9:
	.loc 1 54 10
	movl	-4(%rbp), %eax
	movzbl	-80(%rbp,%rax), %eax
	.loc 1 54 14
	xorl	$54, %eax
	movl	%eax, %edx
	movl	-4(%rbp), %eax
	movb	%dl, -80(%rbp,%rax)
	.loc 1 53 34 discriminator 3
	addl	$1, -4(%rbp)
.L8:
	.loc 1 53 17 discriminator 1
	movl	-4(%rbp), %edx
	movq	hmac_key_len.1(%rip), %rax
	cmpq	%rax, %rdx
	jb	.L9
	.loc 1 57 3
	movq	-88(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Init@PLT
	.loc 1 58 3
	leaq	-80(%rbp), %rcx
	movq	-88(%rbp), %rax
	movl	$64, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 60 3
	movq	hmac_key_len.1(%rip), %rax
	movl	$64, %edx
	subq	%rax, %rdx
	movq	hmac_key_len.1(%rip), %rax
	leaq	-80(%rbp), %rcx
	addq	%rcx, %rax
	movl	$92, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 61 10
	movl	$0, -4(%rbp)
	.loc 1 61 3
	jmp	.L10
.L11:
	.loc 1 62 10
	movl	-4(%rbp), %eax
	movzbl	-80(%rbp,%rax), %eax
	.loc 1 62 14
	xorl	$106, %eax
	movl	%eax, %edx
	movl	-4(%rbp), %eax
	movb	%dl, -80(%rbp,%rax)
	.loc 1 61 34 discriminator 3
	addl	$1, -4(%rbp)
.L10:
	.loc 1 61 17 discriminator 1
	movl	-4(%rbp), %edx
	movq	hmac_key_len.1(%rip), %rax
	cmpq	%rax, %rdx
	jb	.L11
	.loc 1 65 3
	movq	-96(%rbp), %rax
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Init@PLT
	.loc 1 66 3
	leaq	-80(%rbp), %rcx
	movq	-96(%rbp), %rax
	movl	$64, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 67 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE151:
	.size	hmac_init, .-hmac_init
	.section	.rodata
	.align 8
.LC0:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aesctrhmac.c"
	.section	.text.aead_aes_ctr_hmac_sha256_init,"ax",@progbits
	.type	aead_aes_ctr_hmac_sha256_init, @function
aead_aes_ctr_hmac_sha256_init:
.LFB152:
	.loc 1 70 74
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
	.loc 1 71 40
	movq	-24(%rbp), %rax
	addq	$8, %rax
	movq	%rax, -8(%rbp)
	.loc 1 75 15
	movq	hmac_key_len.0(%rip), %rax
	.loc 1 75 6
	cmpq	%rax, -40(%rbp)
	jnb	.L13
	.loc 1 76 5
	movl	$76, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$102, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 77 12
	movl	$0, %eax
	jmp	.L14
.L13:
	.loc 1 80 38
	movq	hmac_key_len.0(%rip), %rax
	.loc 1 80 16
	movq	-40(%rbp), %rdx
	subq	%rax, %rdx
	movq	%rdx, -16(%rbp)
	.loc 1 81 6
	cmpq	$16, -16(%rbp)
	je	.L15
	.loc 1 81 25 discriminator 1
	cmpq	$32, -16(%rbp)
	je	.L15
	.loc 1 82 5
	movl	$82, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$102, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 83 12
	movl	$0, %eax
	jmp	.L14
.L15:
	.loc 1 86 6
	cmpq	$0, -48(%rbp)
	jne	.L16
	.loc 1 87 13
	movq	$32, -48(%rbp)
.L16:
	.loc 1 90 6
	cmpq	$32, -48(%rbp)
	jbe	.L17
	.loc 1 91 5
	movl	$91, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$116, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 92 12
	movl	$0, %eax
	jmp	.L14
.L17:
	.loc 1 96 7
	movq	-8(%rbp), %rax
	leaq	256(%rax), %rsi
	movq	-8(%rbp), %rax
	movq	-16(%rbp), %rcx
	movq	-32(%rbp), %rdx
	movq	%rcx, %r8
	movq	%rdx, %rcx
	movq	%rsi, %rdx
	movl	$0, %esi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_aes_ctr_set_key@PLT
	.loc 1 95 16
	movq	-8(%rbp), %rdx
	movq	%rax, 248(%rdx)
	.loc 1 97 16
	movq	-48(%rbp), %rax
	movl	%eax, %edx
	movq	-24(%rbp), %rax
	movb	%dl, 577(%rax)
	.loc 1 98 3
	movq	-32(%rbp), %rdx
	movq	-16(%rbp), %rax
	addq	%rax, %rdx
	movq	-8(%rbp), %rax
	leaq	376(%rax), %rcx
	movq	-8(%rbp), %rax
	addq	$264, %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	hmac_init
	.loc 1 101 10
	movl	$1, %eax
.L14:
	.loc 1 102 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE152:
	.size	aead_aes_ctr_hmac_sha256_init, .-aead_aes_ctr_hmac_sha256_init
	.section	.text.aead_aes_ctr_hmac_sha256_cleanup,"ax",@progbits
	.type	aead_aes_ctr_hmac_sha256_cleanup, @function
aead_aes_ctr_hmac_sha256_cleanup:
.LFB153:
	.loc 1 104 65
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	.loc 1 104 66
	nop
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE153:
	.size	aead_aes_ctr_hmac_sha256_cleanup, .-aead_aes_ctr_hmac_sha256_cleanup
	.section	.text.hmac_update_uint64,"ax",@progbits
	.type	hmac_update_uint64, @function
hmac_update_uint64:
.LFB154:
	.loc 1 106 68
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$32, %rsp
	movq	%rdi, -24(%rbp)
	movq	%rsi, -32(%rbp)
	.loc 1 110 10
	movl	$0, -4(%rbp)
	.loc 1 110 3
	jmp	.L20
.L21:
	.loc 1 111 14
	movq	-32(%rbp), %rax
	movl	%eax, %edx
	movl	-4(%rbp), %eax
	movb	%dl, -12(%rbp,%rax)
	.loc 1 112 11
	shrq	$8, -32(%rbp)
	.loc 1 110 35 discriminator 3
	addl	$1, -4(%rbp)
.L20:
	.loc 1 110 17 discriminator 1
	cmpl	$7, -4(%rbp)
	jbe	.L21
	.loc 1 114 3
	leaq	-12(%rbp), %rcx
	movq	-24(%rbp), %rax
	movl	$8, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 115 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE154:
	.size	hmac_update_uint64, .-hmac_update_uint64
	.section	.text.hmac_calculate,"ax",@progbits
	.type	hmac_calculate, @function
hmac_calculate:
.LFB155:
	.loc 1 122 51
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$272, %rsp
	movq	%rdi, -232(%rbp)
	movq	%rsi, -240(%rbp)
	movq	%rdx, -248(%rbp)
	movq	%rcx, -256(%rbp)
	movq	%r8, -264(%rbp)
	movq	%r9, -272(%rbp)
	.loc 1 124 3
	movq	-240(%rbp), %rcx
	leaq	-128(%rbp), %rax
	movl	$112, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 125 3
	movq	-264(%rbp), %rdx
	leaq	-128(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	hmac_update_uint64
	.loc 1 126 3
	movq	24(%rbp), %rdx
	leaq	-128(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	hmac_update_uint64
	.loc 1 127 3
	movq	-272(%rbp), %rcx
	leaq	-128(%rbp), %rax
	movl	$12, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 128 3
	movq	-264(%rbp), %rdx
	movq	-256(%rbp), %rcx
	leaq	-128(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 134 41
	movq	-264(%rbp), %rax
	movl	%eax, %edx
	movl	$-28, %eax
	subl	%edx, %eax
	.loc 1 131 18
	andl	$63, %eax
	movl	%eax, -4(%rbp)
	.loc 1 137 3
	movl	-4(%rbp), %edx
	leaq	-192(%rbp), %rax
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 138 3
	movl	-4(%rbp), %edx
	leaq	-192(%rbp), %rcx
	leaq	-128(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 140 3
	movq	24(%rbp), %rdx
	movq	16(%rbp), %rcx
	leaq	-128(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 143 3
	leaq	-128(%rbp), %rdx
	leaq	-224(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Final@PLT
	.loc 1 145 3
	movq	-248(%rbp), %rcx
	leaq	-128(%rbp), %rax
	movl	$112, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 146 3
	leaq	-224(%rbp), %rcx
	leaq	-128(%rbp), %rax
	movl	$32, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Update@PLT
	.loc 1 147 3
	leaq	-128(%rbp), %rdx
	movq	-232(%rbp), %rax
	movq	%rdx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_SHA256_Final@PLT
	.loc 1 148 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE155:
	.size	hmac_calculate, .-hmac_calculate
	.section	.text.aead_aes_ctr_hmac_sha256_crypt,"ax",@progbits
	.type	aead_aes_ctr_hmac_sha256_crypt, @function
aead_aes_ctr_hmac_sha256_crypt:
.LFB156:
	.loc 1 152 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$96, %rsp
	movq	%rdi, -56(%rbp)
	movq	%rsi, -64(%rbp)
	movq	%rdx, -72(%rbp)
	movq	%rcx, -80(%rbp)
	movq	%r8, -88(%rbp)
	.loc 1 156 12
	movl	$0, -20(%rbp)
	.loc 1 157 3
	leaq	-16(%rbp), %rax
	movl	$16, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 160 3
	movq	-88(%rbp), %rcx
	leaq	-48(%rbp), %rax
	movl	$12, %edx
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 161 3
	leaq	-48(%rbp), %rax
	addq	$12, %rax
	movl	$4, %edx
	movl	$0, %esi
	movq	%rax, %rdi
	call	OPENSSL_memset
	.loc 1 163 14
	movq	-56(%rbp), %rax
	movq	248(%rax), %rax
	.loc 1 163 6
	testq	%rax, %rax
	je	.L24
	.loc 1 166 40
	movq	-56(%rbp), %rax
	movq	248(%rax), %rdi
	.loc 1 164 5
	movq	-56(%rbp), %rcx
	leaq	-16(%rbp), %r9
	leaq	-48(%rbp), %r8
	movq	-80(%rbp), %rdx
	movq	-64(%rbp), %rsi
	movq	-72(%rbp), %rax
	pushq	%rdi
	leaq	-20(%rbp), %rdi
	pushq	%rdi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_ctr128_encrypt_ctr32@PLT
	addq	$16, %rsp
	.loc 1 172 1
	jmp	.L26
.L24:
	.loc 1 170 34
	movq	-56(%rbp), %rax
	movq	256(%rax), %rdi
	.loc 1 168 5
	movq	-56(%rbp), %rcx
	leaq	-16(%rbp), %r9
	leaq	-48(%rbp), %r8
	movq	-80(%rbp), %rdx
	movq	-64(%rbp), %rsi
	movq	-72(%rbp), %rax
	pushq	%rdi
	leaq	-20(%rbp), %rdi
	pushq	%rdi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_ctr128_encrypt@PLT
	addq	$16, %rsp
.L26:
	.loc 1 172 1
	nop
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE156:
	.size	aead_aes_ctr_hmac_sha256_crypt, .-aead_aes_ctr_hmac_sha256_crypt
	.section	.text.aead_aes_ctr_hmac_sha256_seal_scatter,"ax",@progbits
	.type	aead_aes_ctr_hmac_sha256_seal_scatter, @function
aead_aes_ctr_hmac_sha256_seal_scatter:
.LFB157:
	.loc 1 178 60
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$96, %rsp
	movq	%rdi, -56(%rbp)
	movq	%rsi, -64(%rbp)
	movq	%rdx, -72(%rbp)
	movq	%rcx, -80(%rbp)
	movq	%r8, -88(%rbp)
	movq	%r9, -96(%rbp)
	.loc 1 179 46
	movq	-56(%rbp), %rax
	addq	$8, %rax
	movq	%rax, -8(%rbp)
	.loc 1 181 18
	movq	32(%rbp), %rax
	movq	%rax, -16(%rbp)
	.loc 1 183 6
	movabsq	$68719476735, %rax
	cmpq	-16(%rbp), %rax
	jnb	.L28
	.loc 1 185 5
	movl	$185, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$117, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 186 12
	movl	$0, %eax
	jmp	.L32
.L28:
	.loc 1 189 28
	movq	-56(%rbp), %rax
	movzbl	577(%rax), %eax
	movzbl	%al, %eax
	.loc 1 189 6
	cmpq	%rax, -88(%rbp)
	jnb	.L30
	.loc 1 190 5
	movl	$190, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$103, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 191 12
	movl	$0, %eax
	jmp	.L32
.L30:
	.loc 1 194 6
	cmpq	$12, 16(%rbp)
	je	.L31
	.loc 1 195 5
	movl	$195, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$121, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 196 12
	movl	$0, %eax
	jmp	.L32
.L31:
	.loc 1 199 3
	movq	-96(%rbp), %rdi
	movq	32(%rbp), %rcx
	movq	24(%rbp), %rdx
	movq	-64(%rbp), %rsi
	movq	-8(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aead_aes_ctr_hmac_sha256_crypt
	.loc 1 202 3
	movq	-8(%rbp), %rax
	leaq	376(%rax), %rdi
	movq	-8(%rbp), %rax
	leaq	264(%rax), %rsi
	movq	-96(%rbp), %r8
	movq	64(%rbp), %rcx
	movq	56(%rbp), %rdx
	leaq	-48(%rbp), %rax
	pushq	32(%rbp)
	pushq	-64(%rbp)
	movq	%r8, %r9
	movq	%rcx, %r8
	movq	%rdx, %rcx
	movq	%rdi, %rdx
	movq	%rax, %rdi
	call	hmac_calculate
	addq	$16, %rsp
	.loc 1 204 43
	movq	-56(%rbp), %rax
	movzbl	577(%rax), %eax
	.loc 1 204 3
	movzbl	%al, %edx
	leaq	-48(%rbp), %rcx
	movq	-72(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	OPENSSL_memcpy
	.loc 1 205 21
	movq	-56(%rbp), %rax
	movzbl	577(%rax), %eax
	movzbl	%al, %edx
	.loc 1 205 16
	movq	-80(%rbp), %rax
	movq	%rdx, (%rax)
	.loc 1 207 10
	movl	$1, %eax
.L32:
	.loc 1 208 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE157:
	.size	aead_aes_ctr_hmac_sha256_seal_scatter, .-aead_aes_ctr_hmac_sha256_seal_scatter
	.section	.text.aead_aes_ctr_hmac_sha256_open_gather,"ax",@progbits
	.type	aead_aes_ctr_hmac_sha256_open_gather, @function
aead_aes_ctr_hmac_sha256_open_gather:
.LFB158:
	.loc 1 213 58
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	subq	$96, %rsp
	movq	%rdi, -56(%rbp)
	movq	%rsi, -64(%rbp)
	movq	%rdx, -72(%rbp)
	movq	%rcx, -80(%rbp)
	movq	%r8, -88(%rbp)
	movq	%r9, -96(%rbp)
	.loc 1 214 46
	movq	-56(%rbp), %rax
	addq	$8, %rax
	movq	%rax, -8(%rbp)
	.loc 1 217 24
	movq	-56(%rbp), %rax
	movzbl	577(%rax), %eax
	movzbl	%al, %eax
	.loc 1 217 6
	cmpq	%rax, 24(%rbp)
	je	.L34
	.loc 1 218 5
	movl	$218, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 219 12
	movl	$0, %eax
	jmp	.L38
.L34:
	.loc 1 222 6
	cmpq	$12, -80(%rbp)
	je	.L36
	.loc 1 223 5
	movl	$223, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$121, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 224 12
	movl	$0, %eax
	jmp	.L38
.L36:
	.loc 1 228 3
	movq	-8(%rbp), %rax
	leaq	376(%rax), %rdi
	movq	-8(%rbp), %rax
	leaq	264(%rax), %rsi
	movq	-72(%rbp), %r8
	movq	40(%rbp), %rcx
	movq	32(%rbp), %rdx
	leaq	-48(%rbp), %rax
	pushq	-96(%rbp)
	pushq	-88(%rbp)
	movq	%r8, %r9
	movq	%rcx, %r8
	movq	%rdx, %rcx
	movq	%rdi, %rdx
	movq	%rax, %rdi
	call	hmac_calculate
	addq	$16, %rsp
	.loc 1 230 45
	movq	-56(%rbp), %rax
	movzbl	577(%rax), %eax
	.loc 1 230 7
	movzbl	%al, %edx
	movq	16(%rbp), %rcx
	leaq	-48(%rbp), %rax
	movq	%rcx, %rsi
	movq	%rax, %rdi
	call	aws_lc_0_38_0_CRYPTO_memcmp@PLT
	.loc 1 230 6 discriminator 1
	testl	%eax, %eax
	je	.L37
	.loc 1 231 5
	movl	$231, %r8d
	leaq	.LC0(%rip), %rax
	movq	%rax, %rcx
	movl	$101, %edx
	movl	$0, %esi
	movl	$30, %edi
	call	aws_lc_0_38_0_ERR_put_error@PLT
	.loc 1 232 12
	movl	$0, %eax
	jmp	.L38
.L37:
	.loc 1 235 3
	movq	-72(%rbp), %rdi
	movq	-96(%rbp), %rcx
	movq	-88(%rbp), %rdx
	movq	-64(%rbp), %rsi
	movq	-8(%rbp), %rax
	movq	%rdi, %r8
	movq	%rax, %rdi
	call	aead_aes_ctr_hmac_sha256_crypt
	.loc 1 237 10
	movl	$1, %eax
.L38:
	.loc 1 238 1
	leave
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE158:
	.size	aead_aes_ctr_hmac_sha256_open_gather, .-aead_aes_ctr_hmac_sha256_open_gather
	.section	.data.rel.ro.local.aead_aes_128_ctr_hmac_sha256,"aw"
	.align 32
	.type	aead_aes_128_ctr_hmac_sha256, @object
	.size	aead_aes_128_ctr_hmac_sha256, 96
aead_aes_128_ctr_hmac_sha256:
	.byte	48
	.byte	12
	.byte	32
	.byte	32
	.value	1
	.zero	2
	.long	0
	.zero	4
	.quad	aead_aes_ctr_hmac_sha256_init
	.quad	0
	.quad	aead_aes_ctr_hmac_sha256_cleanup
	.quad	0
	.quad	aead_aes_ctr_hmac_sha256_seal_scatter
	.quad	aead_aes_ctr_hmac_sha256_open_gather
	.quad	0
	.quad	0
	.quad	0
	.quad	0
	.section	.data.rel.ro.local.aead_aes_256_ctr_hmac_sha256,"aw"
	.align 32
	.type	aead_aes_256_ctr_hmac_sha256, @object
	.size	aead_aes_256_ctr_hmac_sha256, 96
aead_aes_256_ctr_hmac_sha256:
	.byte	64
	.byte	12
	.byte	32
	.byte	32
	.value	2
	.zero	2
	.long	0
	.zero	4
	.quad	aead_aes_ctr_hmac_sha256_init
	.quad	0
	.quad	aead_aes_ctr_hmac_sha256_cleanup
	.quad	0
	.quad	aead_aes_ctr_hmac_sha256_seal_scatter
	.quad	aead_aes_ctr_hmac_sha256_open_gather
	.quad	0
	.quad	0
	.quad	0
	.quad	0
	.section	.text.aws_lc_0_38_0_EVP_aead_aes_128_ctr_hmac_sha256,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_aead_aes_128_ctr_hmac_sha256
	.type	aws_lc_0_38_0_EVP_aead_aes_128_ctr_hmac_sha256, @function
aws_lc_0_38_0_EVP_aead_aes_128_ctr_hmac_sha256:
.LFB159:
	.loc 1 280 56
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 1 281 10
	leaq	aead_aes_128_ctr_hmac_sha256(%rip), %rax
	.loc 1 282 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE159:
	.size	aws_lc_0_38_0_EVP_aead_aes_128_ctr_hmac_sha256, .-aws_lc_0_38_0_EVP_aead_aes_128_ctr_hmac_sha256
	.section	.text.aws_lc_0_38_0_EVP_aead_aes_256_ctr_hmac_sha256,"ax",@progbits
	.globl	aws_lc_0_38_0_EVP_aead_aes_256_ctr_hmac_sha256
	.type	aws_lc_0_38_0_EVP_aead_aes_256_ctr_hmac_sha256, @function
aws_lc_0_38_0_EVP_aead_aes_256_ctr_hmac_sha256:
.LFB160:
	.loc 1 284 56
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	.loc 1 285 10
	leaq	aead_aes_256_ctr_hmac_sha256(%rip), %rax
	.loc 1 286 1
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE160:
	.size	aws_lc_0_38_0_EVP_aead_aes_256_ctr_hmac_sha256, .-aws_lc_0_38_0_EVP_aead_aes_256_ctr_hmac_sha256
	.section	.rodata.hmac_key_len.1,"a"
	.align 8
	.type	hmac_key_len.1, @object
	.size	hmac_key_len.1, 8
hmac_key_len.1:
	.quad	32
	.section	.rodata.hmac_key_len.0,"a"
	.align 8
	.type	hmac_key_len.0, @object
	.size	hmac_key_len.0, 8
hmac_key_len.0:
	.quad	32
	.text
.Letext0:
	.file 3 "/usr/lib/gcc/x86_64-redhat-linux/14/include/stddef.h"
	.file 4 "/usr/include/bits/types.h"
	.file 5 "/usr/include/bits/stdint-uintn.h"
	.file 6 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/bytestring.h"
	.file 7 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/base.h"
	.file 8 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/../fipsmodule/cipher/internal.h"
	.file 9 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/aead.h"
	.file 10 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/sha.h"
	.file 11 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/aes.h"
	.file 12 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/../fipsmodule/cipher/../modes/internal.h"
	.file 13 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/mem.h"
	.file 14 "/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/include/openssl/err.h"
	.file 15 "/usr/include/string.h"
	.section	.debug_info,"",@progbits
.Ldebug_info0:
	.long	0x1075
	.value	0x4
	.long	.Ldebug_abbrev0
	.byte	0x8
	.uleb128 0x1
	.long	.LASF140
	.byte	0xc
	.long	.LASF141
	.long	.LASF142
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
	.uleb128 0x4
	.long	0x6b
	.uleb128 0x2
	.byte	0x1
	.byte	0x6
	.long	.LASF7
	.uleb128 0x3
	.long	.LASF9
	.byte	0x4
	.byte	0x26
	.byte	0x17
	.long	0x5d
	.uleb128 0x2
	.byte	0x2
	.byte	0x5
	.long	.LASF10
	.uleb128 0x3
	.long	.LASF11
	.byte	0x4
	.byte	0x28
	.byte	0x1c
	.long	0x64
	.uleb128 0x3
	.long	.LASF12
	.byte	0x4
	.byte	0x2a
	.byte	0x16
	.long	0x6b
	.uleb128 0x3
	.long	.LASF13
	.byte	0x4
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
	.long	0xb7
	.uleb128 0x3
	.long	.LASF15
	.byte	0x5
	.byte	0x18
	.byte	0x13
	.long	0x7e
	.uleb128 0x4
	.long	0xc3
	.uleb128 0x3
	.long	.LASF16
	.byte	0x5
	.byte	0x19
	.byte	0x14
	.long	0x91
	.uleb128 0x3
	.long	.LASF17
	.byte	0x5
	.byte	0x1a
	.byte	0x14
	.long	0x9d
	.uleb128 0x3
	.long	.LASF18
	.byte	0x5
	.byte	0x1b
	.byte	0x14
	.long	0xa9
	.uleb128 0x4
	.long	0xec
	.uleb128 0x2
	.byte	0x8
	.byte	0x7
	.long	.LASF19
	.uleb128 0x7
	.byte	0x8
	.long	0x10a
	.uleb128 0x8
	.uleb128 0x9
	.string	"CBB"
	.byte	0x7
	.value	0x194
	.byte	0x17
	.long	0x118
	.uleb128 0xa
	.long	.LASF22
	.byte	0x30
	.byte	0x6
	.value	0x1be
	.byte	0x8
	.long	0x14f
	.uleb128 0xb
	.long	.LASF20
	.byte	0x6
	.value	0x1c0
	.byte	0x8
	.long	0x511
	.byte	0
	.uleb128 0xb
	.long	.LASF21
	.byte	0x6
	.value	0x1c3
	.byte	0x8
	.long	0xb7
	.byte	0x8
	.uleb128 0xc
	.string	"u"
	.byte	0x6
	.value	0x1c7
	.byte	0x5
	.long	0x4ec
	.byte	0x10
	.byte	0
	.uleb128 0x9
	.string	"CBS"
	.byte	0x7
	.value	0x195
	.byte	0x17
	.long	0x15c
	.uleb128 0xd
	.long	.LASF23
	.byte	0x10
	.byte	0x6
	.byte	0x28
	.byte	0x8
	.long	0x184
	.uleb128 0xe
	.long	.LASF24
	.byte	0x6
	.byte	0x29
	.byte	0x12
	.long	0x435
	.byte	0
	.uleb128 0xf
	.string	"len"
	.byte	0x6
	.byte	0x2a
	.byte	0xa
	.long	0x30
	.byte	0x8
	.byte	0
	.uleb128 0x10
	.long	.LASF25
	.byte	0x7
	.value	0x1a6
	.byte	0x1c
	.long	0x196
	.uleb128 0x4
	.long	0x184
	.uleb128 0xd
	.long	.LASF26
	.byte	0x60
	.byte	0x8
	.byte	0x71
	.byte	0x8
	.long	0x274
	.uleb128 0xe
	.long	.LASF27
	.byte	0x8
	.byte	0x72
	.byte	0xb
	.long	0xc3
	.byte	0
	.uleb128 0xe
	.long	.LASF28
	.byte	0x8
	.byte	0x73
	.byte	0xb
	.long	0xc3
	.byte	0x1
	.uleb128 0xe
	.long	.LASF29
	.byte	0x8
	.byte	0x74
	.byte	0xb
	.long	0xc3
	.byte	0x2
	.uleb128 0xe
	.long	.LASF30
	.byte	0x8
	.byte	0x75
	.byte	0xb
	.long	0xc3
	.byte	0x3
	.uleb128 0xe
	.long	.LASF31
	.byte	0x8
	.byte	0x76
	.byte	0xc
	.long	0xd4
	.byte	0x4
	.uleb128 0xe
	.long	.LASF32
	.byte	0x8
	.byte	0x77
	.byte	0x7
	.long	0x48
	.byte	0x8
	.uleb128 0xe
	.long	.LASF33
	.byte	0x8
	.byte	0x7b
	.byte	0x9
	.long	0x6ae
	.byte	0x10
	.uleb128 0xe
	.long	.LASF34
	.byte	0x8
	.byte	0x7d
	.byte	0x9
	.long	0x6d7
	.byte	0x18
	.uleb128 0xe
	.long	.LASF35
	.byte	0x8
	.byte	0x7f
	.byte	0xa
	.long	0x6e8
	.byte	0x20
	.uleb128 0xe
	.long	.LASF36
	.byte	0x8
	.byte	0x81
	.byte	0x9
	.long	0x736
	.byte	0x28
	.uleb128 0xe
	.long	.LASF37
	.byte	0x8
	.byte	0x86
	.byte	0x9
	.long	0x787
	.byte	0x30
	.uleb128 0xe
	.long	.LASF38
	.byte	0x8
	.byte	0x8c
	.byte	0x9
	.long	0x7c9
	.byte	0x38
	.uleb128 0xe
	.long	.LASF39
	.byte	0x8
	.byte	0x91
	.byte	0x9
	.long	0x7ee
	.byte	0x40
	.uleb128 0xe
	.long	.LASF40
	.byte	0x8
	.byte	0x94
	.byte	0xc
	.long	0x80d
	.byte	0x48
	.uleb128 0xe
	.long	.LASF41
	.byte	0x8
	.byte	0x97
	.byte	0x9
	.long	0x827
	.byte	0x50
	.uleb128 0xe
	.long	.LASF42
	.byte	0x8
	.byte	0x99
	.byte	0x9
	.long	0x847
	.byte	0x58
	.byte	0
	.uleb128 0x10
	.long	.LASF43
	.byte	0x7
	.value	0x1a7
	.byte	0x20
	.long	0x286
	.uleb128 0x4
	.long	0x274
	.uleb128 0x11
	.long	.LASF44
	.value	0x248
	.byte	0x9
	.byte	0xdd
	.byte	0x8
	.long	0x2cb
	.uleb128 0xe
	.long	.LASF45
	.byte	0x9
	.byte	0xde
	.byte	0x13
	.long	0x379
	.byte	0
	.uleb128 0xe
	.long	.LASF46
	.byte	0x9
	.byte	0xdf
	.byte	0x1f
	.long	0x335
	.byte	0x8
	.uleb128 0x12
	.long	.LASF47
	.byte	0x9
	.byte	0xe0
	.byte	0xb
	.long	0xc3
	.value	0x240
	.uleb128 0x12
	.long	.LASF40
	.byte	0x9
	.byte	0xe3
	.byte	0xb
	.long	0xc3
	.value	0x241
	.byte	0
	.uleb128 0x10
	.long	.LASF48
	.byte	0x7
	.value	0x1d5
	.byte	0x20
	.long	0x2dd
	.uleb128 0x4
	.long	0x2cb
	.uleb128 0xd
	.long	.LASF49
	.byte	0x70
	.byte	0xa
	.byte	0xae
	.byte	0x8
	.long	0x335
	.uleb128 0xf
	.string	"h"
	.byte	0xa
	.byte	0xaf
	.byte	0xc
	.long	0x3d5
	.byte	0
	.uleb128 0xf
	.string	"Nl"
	.byte	0xa
	.byte	0xb0
	.byte	0xc
	.long	0xe0
	.byte	0x20
	.uleb128 0xf
	.string	"Nh"
	.byte	0xa
	.byte	0xb0
	.byte	0x10
	.long	0xe0
	.byte	0x24
	.uleb128 0xe
	.long	.LASF24
	.byte	0xa
	.byte	0xb1
	.byte	0xb
	.long	0x3c5
	.byte	0x28
	.uleb128 0xf
	.string	"num"
	.byte	0xa
	.byte	0xb2
	.byte	0xc
	.long	0x6b
	.byte	0x68
	.uleb128 0xe
	.long	.LASF50
	.byte	0xa
	.byte	0xb2
	.byte	0x11
	.long	0x6b
	.byte	0x6c
	.byte	0
	.uleb128 0x13
	.long	.LASF143
	.value	0x238
	.byte	0x9
	.byte	0xd5
	.byte	0x7
	.long	0x368
	.uleb128 0x14
	.long	.LASF51
	.byte	0x9
	.byte	0xd6
	.byte	0xb
	.long	0x368
	.uleb128 0x14
	.long	.LASF52
	.byte	0x9
	.byte	0xd7
	.byte	0xc
	.long	0xec
	.uleb128 0x15
	.string	"ptr"
	.byte	0x9
	.byte	0xd8
	.byte	0x9
	.long	0xb5
	.byte	0
	.uleb128 0x16
	.long	0xc3
	.long	0x379
	.uleb128 0x17
	.long	0x41
	.value	0x233
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x191
	.uleb128 0x18
	.long	.LASF144
	.byte	0x7
	.byte	0x4
	.long	0x6b
	.byte	0x9
	.value	0x1b6
	.byte	0x6
	.long	0x39f
	.uleb128 0x19
	.long	.LASF53
	.byte	0
	.uleb128 0x19
	.long	.LASF54
	.byte	0x1
	.byte	0
	.uleb128 0x16
	.long	0xc3
	.long	0x3af
	.uleb128 0x1a
	.long	0x41
	.byte	0xf
	.byte	0
	.uleb128 0x16
	.long	0xc3
	.long	0x3bf
	.uleb128 0x1a
	.long	0x41
	.byte	0x1f
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0xbe
	.uleb128 0x16
	.long	0xc3
	.long	0x3d5
	.uleb128 0x1a
	.long	0x41
	.byte	0x3f
	.byte	0
	.uleb128 0x16
	.long	0xe0
	.long	0x3e5
	.uleb128 0x1a
	.long	0x41
	.byte	0x7
	.byte	0
	.uleb128 0x2
	.byte	0x8
	.byte	0x4
	.long	.LASF55
	.uleb128 0xd
	.long	.LASF56
	.byte	0xf4
	.byte	0xb
	.byte	0x48
	.byte	0x8
	.long	0x414
	.uleb128 0xe
	.long	.LASF57
	.byte	0xb
	.byte	0x49
	.byte	0xc
	.long	0x414
	.byte	0
	.uleb128 0xe
	.long	.LASF58
	.byte	0xb
	.byte	0x4a
	.byte	0xc
	.long	0x6b
	.byte	0xf0
	.byte	0
	.uleb128 0x16
	.long	0xe0
	.long	0x424
	.uleb128 0x1a
	.long	0x41
	.byte	0x3b
	.byte	0
	.uleb128 0x3
	.long	.LASF59
	.byte	0xb
	.byte	0x4c
	.byte	0x1b
	.long	0x3ec
	.uleb128 0x4
	.long	0x424
	.uleb128 0x7
	.byte	0x8
	.long	0xcf
	.uleb128 0xa
	.long	.LASF60
	.byte	0x20
	.byte	0x6
	.value	0x1a4
	.byte	0x8
	.long	0x496
	.uleb128 0xc
	.string	"buf"
	.byte	0x6
	.value	0x1a5
	.byte	0xc
	.long	0x496
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
	.uleb128 0x1b
	.long	.LASF61
	.byte	0x6
	.value	0x1ac
	.byte	0xc
	.long	0x6b
	.byte	0x4
	.byte	0x1
	.byte	0x1f
	.byte	0x18
	.uleb128 0x1b
	.long	.LASF62
	.byte	0x6
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
	.long	0xc3
	.uleb128 0xa
	.long	.LASF63
	.byte	0x18
	.byte	0x6
	.value	0x1b2
	.byte	0x8
	.long	0x4e6
	.uleb128 0xb
	.long	.LASF64
	.byte	0x6
	.value	0x1b4
	.byte	0x19
	.long	0x4e6
	.byte	0
	.uleb128 0xb
	.long	.LASF65
	.byte	0x6
	.value	0x1b7
	.byte	0xa
	.long	0x30
	.byte	0x8
	.uleb128 0xb
	.long	.LASF66
	.byte	0x6
	.value	0x1ba
	.byte	0xb
	.long	0xc3
	.byte	0x10
	.uleb128 0x1b
	.long	.LASF67
	.byte	0x6
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
	.long	0x43b
	.uleb128 0x1c
	.byte	0x20
	.byte	0x6
	.value	0x1c4
	.byte	0x3
	.long	0x511
	.uleb128 0x1d
	.long	.LASF64
	.byte	0x6
	.value	0x1c5
	.byte	0x1a
	.long	0x43b
	.uleb128 0x1d
	.long	.LASF20
	.byte	0x6
	.value	0x1c6
	.byte	0x19
	.long	0x49c
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x10b
	.uleb128 0x2
	.byte	0x10
	.byte	0x5
	.long	.LASF68
	.uleb128 0x2
	.byte	0x10
	.byte	0x7
	.long	.LASF69
	.uleb128 0x2
	.byte	0x1
	.byte	0x2
	.long	.LASF70
	.uleb128 0x3
	.long	.LASF71
	.byte	0xc
	.byte	0x50
	.byte	0x10
	.long	0x538
	.uleb128 0x7
	.byte	0x8
	.long	0x53e
	.uleb128 0x1e
	.long	0x553
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x553
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x430
	.uleb128 0x3
	.long	.LASF72
	.byte	0xc
	.byte	0x65
	.byte	0x10
	.long	0x565
	.uleb128 0x7
	.byte	0x8
	.long	0x56b
	.uleb128 0x1e
	.long	0x58a
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x553
	.uleb128 0x1f
	.long	0x435
	.byte	0
	.uleb128 0x20
	.byte	0x10
	.byte	0xc
	.byte	0x85
	.byte	0x9
	.long	0x5ac
	.uleb128 0xf
	.string	"hi"
	.byte	0xc
	.byte	0x85
	.byte	0x1b
	.long	0xec
	.byte	0
	.uleb128 0xf
	.string	"lo"
	.byte	0xc
	.byte	0x85
	.byte	0x1e
	.long	0xec
	.byte	0x8
	.byte	0
	.uleb128 0x3
	.long	.LASF73
	.byte	0xc
	.byte	0x85
	.byte	0x24
	.long	0x58a
	.uleb128 0x4
	.long	0x5ac
	.uleb128 0x3
	.long	.LASF74
	.byte	0xc
	.byte	0x89
	.byte	0x10
	.long	0x5c9
	.uleb128 0x7
	.byte	0x8
	.long	0x5cf
	.uleb128 0x1e
	.long	0x5df
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x5df
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x5b8
	.uleb128 0x3
	.long	.LASF75
	.byte	0xc
	.byte	0x8e
	.byte	0x10
	.long	0x5f1
	.uleb128 0x7
	.byte	0x8
	.long	0x5f7
	.uleb128 0x1e
	.long	0x611
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x5df
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.byte	0
	.uleb128 0x11
	.long	.LASF76
	.value	0x120
	.byte	0xc
	.byte	0x91
	.byte	0x10
	.long	0x668
	.uleb128 0xe
	.long	.LASF77
	.byte	0xc
	.byte	0x96
	.byte	0x8
	.long	0x668
	.byte	0
	.uleb128 0x12
	.long	.LASF78
	.byte	0xc
	.byte	0x97
	.byte	0xe
	.long	0x5bd
	.value	0x100
	.uleb128 0x12
	.long	.LASF79
	.byte	0xc
	.byte	0x98
	.byte	0xe
	.long	0x5e5
	.value	0x108
	.uleb128 0x12
	.long	.LASF80
	.byte	0xc
	.byte	0x9a
	.byte	0xe
	.long	0x52c
	.value	0x110
	.uleb128 0x21
	.long	.LASF81
	.byte	0xc
	.byte	0x9e
	.byte	0xc
	.long	0x6b
	.byte	0x4
	.byte	0x1
	.byte	0x1f
	.value	0x118
	.byte	0
	.uleb128 0x16
	.long	0x5ac
	.long	0x678
	.uleb128 0x1a
	.long	0x41
	.byte	0xf
	.byte	0
	.uleb128 0x3
	.long	.LASF82
	.byte	0xc
	.byte	0x9f
	.byte	0x3
	.long	0x611
	.uleb128 0x7
	.byte	0x8
	.long	0x424
	.uleb128 0x22
	.long	0x48
	.long	0x6a8
	.uleb128 0x1f
	.long	0x6a8
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x274
	.uleb128 0x7
	.byte	0x8
	.long	0x68a
	.uleb128 0x22
	.long	0x48
	.long	0x6d7
	.uleb128 0x1f
	.long	0x6a8
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x37f
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x6b4
	.uleb128 0x1e
	.long	0x6e8
	.uleb128 0x1f
	.long	0x6a8
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x6dd
	.uleb128 0x22
	.long	0x48
	.long	0x72a
	.uleb128 0x1f
	.long	0x72a
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x730
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x281
	.uleb128 0x7
	.byte	0x8
	.long	0x30
	.uleb128 0x7
	.byte	0x8
	.long	0x6ee
	.uleb128 0x22
	.long	0x48
	.long	0x787
	.uleb128 0x1f
	.long	0x72a
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x730
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x73c
	.uleb128 0x22
	.long	0x48
	.long	0x7c9
	.uleb128 0x1f
	.long	0x72a
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x78d
	.uleb128 0x22
	.long	0x48
	.long	0x7e8
	.uleb128 0x1f
	.long	0x72a
	.uleb128 0x1f
	.long	0x7e8
	.uleb128 0x1f
	.long	0x730
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x435
	.uleb128 0x7
	.byte	0x8
	.long	0x7cf
	.uleb128 0x22
	.long	0x30
	.long	0x80d
	.uleb128 0x1f
	.long	0x72a
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x7f4
	.uleb128 0x22
	.long	0x48
	.long	0x827
	.uleb128 0x1f
	.long	0x72a
	.uleb128 0x1f
	.long	0x511
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x813
	.uleb128 0x22
	.long	0x48
	.long	0x841
	.uleb128 0x1f
	.long	0x72a
	.uleb128 0x1f
	.long	0x841
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x14f
	.uleb128 0x7
	.byte	0x8
	.long	0x82d
	.uleb128 0x23
	.byte	0xf8
	.byte	0x1
	.byte	0x1c
	.byte	0x3
	.long	0x86e
	.uleb128 0x14
	.long	.LASF83
	.byte	0x1
	.byte	0x1d
	.byte	0xc
	.long	0x3e5
	.uleb128 0x15
	.string	"ks"
	.byte	0x1
	.byte	0x1e
	.byte	0xd
	.long	0x424
	.byte	0
	.uleb128 0x11
	.long	.LASF84
	.value	0x1e8
	.byte	0x1
	.byte	0x1b
	.byte	0x8
	.long	0x8c0
	.uleb128 0xf
	.string	"ks"
	.byte	0x1
	.byte	0x1f
	.byte	0x5
	.long	0x84d
	.byte	0
	.uleb128 0xf
	.string	"ctr"
	.byte	0x1
	.byte	0x20
	.byte	0xc
	.long	0x559
	.byte	0xf8
	.uleb128 0x12
	.long	.LASF80
	.byte	0x1
	.byte	0x21
	.byte	0xe
	.long	0x52c
	.value	0x100
	.uleb128 0x12
	.long	.LASF85
	.byte	0x1
	.byte	0x22
	.byte	0xe
	.long	0x2cb
	.value	0x108
	.uleb128 0x12
	.long	.LASF86
	.byte	0x1
	.byte	0x23
	.byte	0xe
	.long	0x2cb
	.value	0x178
	.byte	0
	.uleb128 0x4
	.long	0x86e
	.uleb128 0x24
	.long	.LASF87
	.byte	0x1
	.byte	0xf0
	.byte	0x17
	.long	0x191
	.uleb128 0x9
	.byte	0x3
	.quad	aead_aes_128_ctr_hmac_sha256
	.uleb128 0x25
	.long	.LASF88
	.byte	0x1
	.value	0x104
	.byte	0x17
	.long	0x191
	.uleb128 0x9
	.byte	0x3
	.quad	aead_aes_256_ctr_hmac_sha256
	.uleb128 0x26
	.long	.LASF91
	.byte	0xd
	.byte	0x74
	.byte	0x14
	.long	0x48
	.long	0x912
	.uleb128 0x1f
	.long	0x104
	.uleb128 0x1f
	.long	0x104
	.uleb128 0x1f
	.long	0x30
	.byte	0
	.uleb128 0x27
	.long	.LASF89
	.byte	0xc
	.byte	0x6e
	.byte	0x6
	.long	0x947
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x553
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x947
	.uleb128 0x1f
	.long	0x52c
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x6b
	.uleb128 0x27
	.long	.LASF90
	.byte	0xc
	.byte	0x77
	.byte	0x6
	.long	0x982
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x30
	.uleb128 0x1f
	.long	0x553
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x947
	.uleb128 0x1f
	.long	0x559
	.byte	0
	.uleb128 0x26
	.long	.LASF92
	.byte	0xa
	.byte	0x97
	.byte	0x14
	.long	0x48
	.long	0x99d
	.uleb128 0x1f
	.long	0x496
	.uleb128 0x1f
	.long	0x99d
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x2cb
	.uleb128 0x26
	.long	.LASF93
	.byte	0x8
	.byte	0xc5
	.byte	0xa
	.long	0x559
	.long	0x9cd
	.uleb128 0x1f
	.long	0x684
	.uleb128 0x1f
	.long	0x9cd
	.uleb128 0x1f
	.long	0x9d3
	.uleb128 0x1f
	.long	0x435
	.uleb128 0x1f
	.long	0x30
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x678
	.uleb128 0x7
	.byte	0x8
	.long	0x52c
	.uleb128 0x28
	.long	.LASF94
	.byte	0xe
	.value	0x1d0
	.byte	0x15
	.long	0xa00
	.uleb128 0x1f
	.long	0x48
	.uleb128 0x1f
	.long	0x48
	.uleb128 0x1f
	.long	0x48
	.uleb128 0x1f
	.long	0x3bf
	.uleb128 0x1f
	.long	0x6b
	.byte	0
	.uleb128 0x26
	.long	.LASF95
	.byte	0xf
	.byte	0x3d
	.byte	0xe
	.long	0xb5
	.long	0xa20
	.uleb128 0x1f
	.long	0xb5
	.uleb128 0x1f
	.long	0x48
	.uleb128 0x1f
	.long	0x41
	.byte	0
	.uleb128 0x26
	.long	.LASF96
	.byte	0xf
	.byte	0x2b
	.byte	0xe
	.long	0xb5
	.long	0xa40
	.uleb128 0x1f
	.long	0xb5
	.uleb128 0x1f
	.long	0x104
	.uleb128 0x1f
	.long	0x41
	.byte	0
	.uleb128 0x26
	.long	.LASF97
	.byte	0xa
	.byte	0x92
	.byte	0x14
	.long	0x48
	.long	0xa60
	.uleb128 0x1f
	.long	0x99d
	.uleb128 0x1f
	.long	0x104
	.uleb128 0x1f
	.long	0x30
	.byte	0
	.uleb128 0x26
	.long	.LASF98
	.byte	0xa
	.byte	0x8f
	.byte	0x14
	.long	0x48
	.long	0xa76
	.uleb128 0x1f
	.long	0x99d
	.byte	0
	.uleb128 0x29
	.long	.LASF99
	.byte	0x1
	.value	0x11c
	.byte	0x11
	.long	0x379
	.quad	.LFB160
	.quad	.LFE160-.LFB160
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x29
	.long	.LASF100
	.byte	0x1
	.value	0x118
	.byte	0x11
	.long	0x379
	.quad	.LFB159
	.quad	.LFE159-.LFB159
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x2a
	.long	.LASF108
	.byte	0x1
	.byte	0xd2
	.byte	0xc
	.long	0x48
	.quad	.LFB158
	.quad	.LFE158-.LFB158
	.uleb128 0x1
	.byte	0x9c
	.long	0xb8f
	.uleb128 0x2b
	.string	"ctx"
	.byte	0x1
	.byte	0xd3
	.byte	0x19
	.long	0x72a
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x2b
	.string	"out"
	.byte	0x1
	.byte	0xd3
	.byte	0x27
	.long	0x496
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x2c
	.long	.LASF101
	.byte	0x1
	.byte	0xd3
	.byte	0x3b
	.long	0x435
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x2c
	.long	.LASF28
	.byte	0x1
	.byte	0xd4
	.byte	0xc
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x2b
	.string	"in"
	.byte	0x1
	.byte	0xd4
	.byte	0x26
	.long	0x435
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x2c
	.long	.LASF102
	.byte	0x1
	.byte	0xd4
	.byte	0x31
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x2c
	.long	.LASF103
	.byte	0x1
	.byte	0xd4
	.byte	0x48
	.long	0x435
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x2c
	.long	.LASF104
	.byte	0x1
	.byte	0xd5
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 8
	.uleb128 0x2b
	.string	"ad"
	.byte	0x1
	.byte	0xd5
	.byte	0x27
	.long	0x435
	.uleb128 0x2
	.byte	0x91
	.sleb128 16
	.uleb128 0x2c
	.long	.LASF105
	.byte	0x1
	.byte	0xd5
	.byte	0x32
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 24
	.uleb128 0x24
	.long	.LASF106
	.byte	0x1
	.byte	0xd6
	.byte	0x2e
	.long	0xb8f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x24
	.long	.LASF107
	.byte	0x1
	.byte	0xe3
	.byte	0xb
	.long	0x3af
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x8c0
	.uleb128 0x2a
	.long	.LASF109
	.byte	0x1
	.byte	0xae
	.byte	0xc
	.long	0x48
	.quad	.LFB157
	.quad	.LFE157-.LFB157
	.uleb128 0x1
	.byte	0x9c
	.long	0xcac
	.uleb128 0x2b
	.string	"ctx"
	.byte	0x1
	.byte	0xaf
	.byte	0x19
	.long	0x72a
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x2b
	.string	"out"
	.byte	0x1
	.byte	0xaf
	.byte	0x27
	.long	0x496
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x2c
	.long	.LASF110
	.byte	0x1
	.byte	0xaf
	.byte	0x35
	.long	0x496
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x2c
	.long	.LASF111
	.byte	0x1
	.byte	0xb0
	.byte	0xd
	.long	0x730
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x2c
	.long	.LASF112
	.byte	0x1
	.byte	0xb0
	.byte	0x21
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x2c
	.long	.LASF101
	.byte	0x1
	.byte	0xb0
	.byte	0x41
	.long	0x435
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x2c
	.long	.LASF28
	.byte	0x1
	.byte	0xb1
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x2b
	.string	"in"
	.byte	0x1
	.byte	0xb1
	.byte	0x26
	.long	0x435
	.uleb128 0x2
	.byte	0x91
	.sleb128 8
	.uleb128 0x2c
	.long	.LASF102
	.byte	0x1
	.byte	0xb1
	.byte	0x31
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 16
	.uleb128 0x2c
	.long	.LASF113
	.byte	0x1
	.byte	0xb1
	.byte	0x48
	.long	0x435
	.uleb128 0x2
	.byte	0x91
	.sleb128 24
	.uleb128 0x2c
	.long	.LASF114
	.byte	0x1
	.byte	0xb2
	.byte	0xc
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 32
	.uleb128 0x2b
	.string	"ad"
	.byte	0x1
	.byte	0xb2
	.byte	0x29
	.long	0x435
	.uleb128 0x2
	.byte	0x91
	.sleb128 40
	.uleb128 0x2c
	.long	.LASF105
	.byte	0x1
	.byte	0xb2
	.byte	0x34
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 48
	.uleb128 0x24
	.long	.LASF106
	.byte	0x1
	.byte	0xb3
	.byte	0x2e
	.long	0xb8f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x24
	.long	.LASF115
	.byte	0x1
	.byte	0xb5
	.byte	0x12
	.long	0xf8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x24
	.long	.LASF107
	.byte	0x1
	.byte	0xc9
	.byte	0xb
	.long	0x3af
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.byte	0
	.uleb128 0x2d
	.long	.LASF119
	.byte	0x1
	.byte	0x96
	.byte	0xd
	.quad	.LFB156
	.quad	.LFE156-.LFB156
	.uleb128 0x1
	.byte	0x9c
	.long	0xd47
	.uleb128 0x2c
	.long	.LASF106
	.byte	0x1
	.byte	0x97
	.byte	0x30
	.long	0xb8f
	.uleb128 0x3
	.byte	0x91
	.sleb128 -72
	.uleb128 0x2b
	.string	"out"
	.byte	0x1
	.byte	0x97
	.byte	0x42
	.long	0x496
	.uleb128 0x3
	.byte	0x91
	.sleb128 -80
	.uleb128 0x2b
	.string	"in"
	.byte	0x1
	.byte	0x98
	.byte	0x14
	.long	0x435
	.uleb128 0x3
	.byte	0x91
	.sleb128 -88
	.uleb128 0x2b
	.string	"len"
	.byte	0x1
	.byte	0x98
	.byte	0x1f
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x2c
	.long	.LASF101
	.byte	0x1
	.byte	0x98
	.byte	0x33
	.long	0x435
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x24
	.long	.LASF116
	.byte	0x1
	.byte	0x9b
	.byte	0xb
	.long	0x39f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x24
	.long	.LASF117
	.byte	0x1
	.byte	0x9c
	.byte	0xc
	.long	0x6b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -36
	.uleb128 0x24
	.long	.LASF118
	.byte	0x1
	.byte	0x9f
	.byte	0xb
	.long	0x39f
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.byte	0
	.uleb128 0x2d
	.long	.LASF120
	.byte	0x1
	.byte	0x75
	.byte	0xd
	.quad	.LFB155
	.quad	.LFE155-.LFB155
	.uleb128 0x1
	.byte	0x9c
	.long	0xe22
	.uleb128 0x2b
	.string	"out"
	.byte	0x1
	.byte	0x75
	.byte	0x24
	.long	0x496
	.uleb128 0x3
	.byte	0x91
	.sleb128 -248
	.uleb128 0x2c
	.long	.LASF85
	.byte	0x1
	.byte	0x76
	.byte	0x2e
	.long	0xe22
	.uleb128 0x3
	.byte	0x91
	.sleb128 -256
	.uleb128 0x2c
	.long	.LASF86
	.byte	0x1
	.byte	0x77
	.byte	0x2e
	.long	0xe22
	.uleb128 0x3
	.byte	0x91
	.sleb128 -264
	.uleb128 0x2b
	.string	"ad"
	.byte	0x1
	.byte	0x78
	.byte	0x2b
	.long	0x435
	.uleb128 0x3
	.byte	0x91
	.sleb128 -272
	.uleb128 0x2c
	.long	.LASF105
	.byte	0x1
	.byte	0x78
	.byte	0x36
	.long	0x30
	.uleb128 0x3
	.byte	0x91
	.sleb128 -280
	.uleb128 0x2c
	.long	.LASF101
	.byte	0x1
	.byte	0x79
	.byte	0x2b
	.long	0x435
	.uleb128 0x3
	.byte	0x91
	.sleb128 -288
	.uleb128 0x2c
	.long	.LASF121
	.byte	0x1
	.byte	0x79
	.byte	0x41
	.long	0x435
	.uleb128 0x2
	.byte	0x91
	.sleb128 0
	.uleb128 0x2c
	.long	.LASF122
	.byte	0x1
	.byte	0x7a
	.byte	0x23
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 8
	.uleb128 0x24
	.long	.LASF123
	.byte	0x1
	.byte	0x7b
	.byte	0xe
	.long	0x2cb
	.uleb128 0x3
	.byte	0x91
	.sleb128 -144
	.uleb128 0x24
	.long	.LASF124
	.byte	0x1
	.byte	0x83
	.byte	0x12
	.long	0x72
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x24
	.long	.LASF125
	.byte	0x1
	.byte	0x88
	.byte	0xb
	.long	0x3c5
	.uleb128 0x3
	.byte	0x91
	.sleb128 -208
	.uleb128 0x24
	.long	.LASF126
	.byte	0x1
	.byte	0x8e
	.byte	0xb
	.long	0x3af
	.uleb128 0x3
	.byte	0x91
	.sleb128 -240
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x2d8
	.uleb128 0x2d
	.long	.LASF127
	.byte	0x1
	.byte	0x6a
	.byte	0xd
	.quad	.LFB154
	.quad	.LFE154-.LFB154
	.uleb128 0x1
	.byte	0x9c
	.long	0xe81
	.uleb128 0x2c
	.long	.LASF123
	.byte	0x1
	.byte	0x6a
	.byte	0x2c
	.long	0x99d
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x2c
	.long	.LASF128
	.byte	0x1
	.byte	0x6a
	.byte	0x3d
	.long	0xec
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x2e
	.string	"i"
	.byte	0x1
	.byte	0x6b
	.byte	0xc
	.long	0x6b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.uleb128 0x24
	.long	.LASF129
	.byte	0x1
	.byte	0x6c
	.byte	0xb
	.long	0xe81
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.byte	0
	.uleb128 0x16
	.long	0xc3
	.long	0xe91
	.uleb128 0x1a
	.long	0x41
	.byte	0x7
	.byte	0
	.uleb128 0x2f
	.long	.LASF130
	.byte	0x1
	.byte	0x68
	.byte	0xd
	.quad	.LFB153
	.quad	.LFE153-.LFB153
	.uleb128 0x1
	.byte	0x9c
	.long	0xebf
	.uleb128 0x2b
	.string	"ctx"
	.byte	0x1
	.byte	0x68
	.byte	0x3c
	.long	0x6a8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.byte	0
	.uleb128 0x2a
	.long	.LASF131
	.byte	0x1
	.byte	0x45
	.byte	0xc
	.long	0x48
	.quad	.LFB152
	.quad	.LFE152-.LFB152
	.uleb128 0x1
	.byte	0x9c
	.long	0xf52
	.uleb128 0x2b
	.string	"ctx"
	.byte	0x1
	.byte	0x45
	.byte	0x38
	.long	0x6a8
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.uleb128 0x2b
	.string	"key"
	.byte	0x1
	.byte	0x45
	.byte	0x4c
	.long	0x435
	.uleb128 0x2
	.byte	0x91
	.sleb128 -48
	.uleb128 0x2c
	.long	.LASF27
	.byte	0x1
	.byte	0x46
	.byte	0x31
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -56
	.uleb128 0x2c
	.long	.LASF40
	.byte	0x1
	.byte	0x46
	.byte	0x41
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -64
	.uleb128 0x24
	.long	.LASF106
	.byte	0x1
	.byte	0x47
	.byte	0x28
	.long	0xf52
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x24
	.long	.LASF132
	.byte	0x1
	.byte	0x49
	.byte	0x17
	.long	0x3c
	.uleb128 0x9
	.byte	0x3
	.quad	hmac_key_len.0
	.uleb128 0x24
	.long	.LASF133
	.byte	0x1
	.byte	0x50
	.byte	0x10
	.long	0x3c
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.byte	0
	.uleb128 0x7
	.byte	0x8
	.long	0x86e
	.uleb128 0x2d
	.long	.LASF134
	.byte	0x1
	.byte	0x2d
	.byte	0xd
	.quad	.LFB151
	.quad	.LFE151-.LFB151
	.uleb128 0x1
	.byte	0x9c
	.long	0xfda
	.uleb128 0x2c
	.long	.LASF135
	.byte	0x1
	.byte	0x2d
	.byte	0x23
	.long	0x99d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -104
	.uleb128 0x2c
	.long	.LASF136
	.byte	0x1
	.byte	0x2d
	.byte	0x3a
	.long	0x99d
	.uleb128 0x3
	.byte	0x91
	.sleb128 -112
	.uleb128 0x2c
	.long	.LASF137
	.byte	0x1
	.byte	0x2e
	.byte	0x25
	.long	0x435
	.uleb128 0x3
	.byte	0x91
	.sleb128 -120
	.uleb128 0x24
	.long	.LASF132
	.byte	0x1
	.byte	0x2f
	.byte	0x17
	.long	0x3c
	.uleb128 0x9
	.byte	0x3
	.quad	hmac_key_len.1
	.uleb128 0x24
	.long	.LASF80
	.byte	0x1
	.byte	0x30
	.byte	0xb
	.long	0x3c5
	.uleb128 0x3
	.byte	0x91
	.sleb128 -96
	.uleb128 0x2e
	.string	"i"
	.byte	0x1
	.byte	0x34
	.byte	0xc
	.long	0x6b
	.uleb128 0x2
	.byte	0x91
	.sleb128 -20
	.byte	0
	.uleb128 0x30
	.long	.LASF138
	.byte	0x2
	.value	0x3cc
	.byte	0x15
	.long	0xb5
	.quad	.LFB96
	.quad	.LFE96-.LFB96
	.uleb128 0x1
	.byte	0x9c
	.long	0x102a
	.uleb128 0x31
	.string	"dst"
	.byte	0x2
	.value	0x3cc
	.byte	0x2a
	.long	0xb5
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x31
	.string	"c"
	.byte	0x2
	.value	0x3cc
	.byte	0x33
	.long	0x48
	.uleb128 0x2
	.byte	0x91
	.sleb128 -28
	.uleb128 0x31
	.string	"n"
	.byte	0x2
	.value	0x3cc
	.byte	0x3d
	.long	0x30
	.uleb128 0x2
	.byte	0x91
	.sleb128 -40
	.byte	0
	.uleb128 0x32
	.long	.LASF139
	.byte	0x2
	.value	0x3bc
	.byte	0x15
	.long	0xb5
	.quad	.LFB94
	.quad	.LFE94-.LFB94
	.uleb128 0x1
	.byte	0x9c
	.uleb128 0x31
	.string	"dst"
	.byte	0x2
	.value	0x3bc
	.byte	0x2a
	.long	0xb5
	.uleb128 0x2
	.byte	0x91
	.sleb128 -24
	.uleb128 0x31
	.string	"src"
	.byte	0x2
	.value	0x3bc
	.byte	0x3b
	.long	0x104
	.uleb128 0x2
	.byte	0x91
	.sleb128 -32
	.uleb128 0x31
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
	.uleb128 0x15
	.byte	0x1
	.uleb128 0x27
	.uleb128 0x19
	.uleb128 0x1
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x1f
	.uleb128 0x5
	.byte	0
	.uleb128 0x49
	.uleb128 0x13
	.byte	0
	.byte	0
	.uleb128 0x20
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
	.uleb128 0x21
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
	.uleb128 0x22
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
	.uleb128 0x23
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
	.uleb128 0x24
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
	.uleb128 0x25
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
	.uleb128 0x26
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
	.uleb128 0x27
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
	.uleb128 0x28
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
	.uleb128 0x29
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
	.uleb128 0x2117
	.uleb128 0x19
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
	.uleb128 0x2d
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
	.uleb128 0x2e
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
	.uleb128 0x2f
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
	.uleb128 0x30
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
	.uleb128 0x31
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
	.byte	0
	.byte	0
	.byte	0
	.section	.debug_aranges,"",@progbits
	.long	0xdc
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
	.quad	0
	.quad	0
	.section	.debug_ranges,"",@progbits
.Ldebug_ranges0:
	.quad	.LFB94
	.quad	.LFE94
	.quad	.LFB96
	.quad	.LFE96
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
	.quad	0
	.quad	0
	.section	.debug_line,"",@progbits
.Ldebug_line0:
	.section	.debug_str,"MS",@progbits,1
.LASF107:
	.string	"hmac_result"
.LASF125:
	.string	"padding"
.LASF109:
	.string	"aead_aes_ctr_hmac_sha256_seal_scatter"
.LASF93:
	.string	"aws_lc_0_38_0_aes_ctr_set_key"
.LASF8:
	.string	"size_t"
.LASF86:
	.string	"outer_init_state"
.LASF50:
	.string	"md_len"
.LASF56:
	.string	"aes_key_st"
.LASF63:
	.string	"cbb_child_st"
.LASF18:
	.string	"uint64_t"
.LASF9:
	.string	"__uint8_t"
.LASF66:
	.string	"pending_len_len"
.LASF44:
	.string	"evp_aead_ctx_st"
.LASF27:
	.string	"key_len"
.LASF51:
	.string	"opaque"
.LASF21:
	.string	"is_child"
.LASF40:
	.string	"tag_len"
.LASF24:
	.string	"data"
.LASF139:
	.string	"OPENSSL_memcpy"
.LASF83:
	.string	"align"
.LASF30:
	.string	"max_tag_len"
.LASF100:
	.string	"aws_lc_0_38_0_EVP_aead_aes_128_ctr_hmac_sha256"
.LASF121:
	.string	"ciphertext"
.LASF7:
	.string	"signed char"
.LASF28:
	.string	"nonce_len"
.LASF61:
	.string	"can_resize"
.LASF134:
	.string	"hmac_init"
.LASF144:
	.string	"evp_aead_direction_t"
.LASF90:
	.string	"aws_lc_0_38_0_CRYPTO_ctr128_encrypt_ctr32"
.LASF120:
	.string	"hmac_calculate"
.LASF76:
	.string	"gcm128_key_st"
.LASF46:
	.string	"state"
.LASF0:
	.string	"long int"
.LASF37:
	.string	"seal_scatter"
.LASF131:
	.string	"aead_aes_ctr_hmac_sha256_init"
.LASF136:
	.string	"out_outer"
.LASF92:
	.string	"aws_lc_0_38_0_SHA256_Final"
.LASF2:
	.string	"long long int"
.LASF16:
	.string	"uint16_t"
.LASF89:
	.string	"aws_lc_0_38_0_CRYPTO_ctr128_encrypt"
.LASF55:
	.string	"double"
.LASF58:
	.string	"rounds"
.LASF108:
	.string	"aead_aes_ctr_hmac_sha256_open_gather"
.LASF129:
	.string	"bytes"
.LASF12:
	.string	"__uint32_t"
.LASF116:
	.string	"partial_block_buffer"
.LASF105:
	.string	"ad_len"
.LASF135:
	.string	"out_inner"
.LASF137:
	.string	"hmac_key"
.LASF97:
	.string	"aws_lc_0_38_0_SHA256_Update"
.LASF117:
	.string	"partial_block_offset"
.LASF19:
	.string	"long long unsigned int"
.LASF128:
	.string	"value"
.LASF141:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0/aws-lc/crypto/cipher_extra/e_aesctrhmac.c"
.LASF127:
	.string	"hmac_update_uint64"
.LASF114:
	.string	"extra_in_len"
.LASF80:
	.string	"block"
.LASF6:
	.string	"unsigned int"
.LASF47:
	.string	"state_offset"
.LASF68:
	.string	"__int128"
.LASF34:
	.string	"init_with_direction"
.LASF1:
	.string	"long unsigned int"
.LASF41:
	.string	"serialize_state"
.LASF98:
	.string	"aws_lc_0_38_0_SHA256_Init"
.LASF49:
	.string	"sha256_state_st"
.LASF81:
	.string	"use_hw_gcm_crypt"
.LASF143:
	.string	"evp_aead_ctx_st_state"
.LASF130:
	.string	"aead_aes_ctr_hmac_sha256_cleanup"
.LASF57:
	.string	"rd_key"
.LASF104:
	.string	"in_tag_len"
.LASF25:
	.string	"EVP_AEAD"
.LASF78:
	.string	"gmult"
.LASF132:
	.string	"hmac_key_len"
.LASF64:
	.string	"base"
.LASF87:
	.string	"aead_aes_128_ctr_hmac_sha256"
.LASF45:
	.string	"aead"
.LASF26:
	.string	"evp_aead_st"
.LASF36:
	.string	"open"
.LASF22:
	.string	"cbb_st"
.LASF102:
	.string	"in_len"
.LASF79:
	.string	"ghash"
.LASF29:
	.string	"overhead"
.LASF103:
	.string	"in_tag"
.LASF35:
	.string	"cleanup"
.LASF71:
	.string	"block128_f"
.LASF38:
	.string	"open_gather"
.LASF33:
	.string	"init"
.LASF13:
	.string	"__uint64_t"
.LASF84:
	.string	"aead_aes_ctr_hmac_sha256_ctx"
.LASF67:
	.string	"pending_is_asn1"
.LASF94:
	.string	"aws_lc_0_38_0_ERR_put_error"
.LASF52:
	.string	"alignment"
.LASF124:
	.string	"num_padding"
.LASF69:
	.string	"__int128 unsigned"
.LASF70:
	.string	"_Bool"
.LASF4:
	.string	"unsigned char"
.LASF142:
	.string	"/home/unnamed/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.38.0"
.LASF20:
	.string	"child"
.LASF23:
	.string	"cbs_st"
.LASF85:
	.string	"inner_init_state"
.LASF10:
	.string	"short int"
.LASF140:
	.string	"GNU C11 14.3.1 20250617 (Red Hat 14.3.1-2) -m64 -mtune=generic -march=x86-64-v3 -g -gdwarf-4 -O0 -std=c11 -ffunction-sections -fdata-sections -fPIC -fno-omit-frame-pointer"
.LASF62:
	.string	"error"
.LASF101:
	.string	"nonce"
.LASF48:
	.string	"SHA256_CTX"
.LASF123:
	.string	"sha256"
.LASF73:
	.string	"u128"
.LASF17:
	.string	"uint32_t"
.LASF77:
	.string	"Htable"
.LASF3:
	.string	"long double"
.LASF14:
	.string	"char"
.LASF138:
	.string	"OPENSSL_memset"
.LASF126:
	.string	"inner_digest"
.LASF11:
	.string	"__uint16_t"
.LASF115:
	.string	"in_len_64"
.LASF42:
	.string	"deserialize_state"
.LASF118:
	.string	"counter"
.LASF119:
	.string	"aead_aes_ctr_hmac_sha256_crypt"
.LASF65:
	.string	"offset"
.LASF96:
	.string	"memcpy"
.LASF43:
	.string	"EVP_AEAD_CTX"
.LASF122:
	.string	"ciphertext_len"
.LASF5:
	.string	"short unsigned int"
.LASF31:
	.string	"aead_id"
.LASF99:
	.string	"aws_lc_0_38_0_EVP_aead_aes_256_ctr_hmac_sha256"
.LASF82:
	.string	"GCM128_KEY"
.LASF95:
	.string	"memset"
.LASF88:
	.string	"aead_aes_256_ctr_hmac_sha256"
.LASF113:
	.string	"extra_in"
.LASF111:
	.string	"out_tag_len"
.LASF15:
	.string	"uint8_t"
.LASF72:
	.string	"ctr128_f"
.LASF39:
	.string	"get_iv"
.LASF75:
	.string	"ghash_func"
.LASF53:
	.string	"evp_aead_open"
.LASF32:
	.string	"seal_scatter_supports_extra_in"
.LASF112:
	.string	"max_out_tag_len"
.LASF59:
	.string	"AES_KEY"
.LASF54:
	.string	"evp_aead_seal"
.LASF60:
	.string	"cbb_buffer_st"
.LASF110:
	.string	"out_tag"
.LASF106:
	.string	"aes_ctx"
.LASF133:
	.string	"aes_key_len"
.LASF91:
	.string	"aws_lc_0_38_0_CRYPTO_memcmp"
.LASF74:
	.string	"gmult_func"
	.ident	"GCC: (GNU) 14.3.1 20250617 (Red Hat 14.3.1-2)"
	.section	.note.GNU-stack,"",@progbits
