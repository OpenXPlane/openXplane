/* Independent x86 instruction oracle; contains no X-Plane code or data. */
#include <stdint.h>
#include <stdio.h>
static uint32_t scalar(uint32_t a,uint32_t b,unsigned op){
    uint32_t r;
#define EVAL(insn) __asm__ volatile("movd %1, %%xmm1; movd %2, %%xmm6; " insn " %%xmm6, %%xmm1; movd %%xmm1, %0":"=r"(r):"r"(a),"r"(b):"xmm1","xmm6")
    if(op==0){EVAL("mulss");}else if(op==1){EVAL("divss");}else{EVAL("addss");}
#undef EVAL
    return r;
}
int main(void){
    uint32_t saved,nearest=0x1f80;
    __asm__ volatile("stmxcsr %0":"=m"(saved));
    __asm__ volatile("ldmxcsr %0"::"m"(nearest));
    uint32_t v[]={0,0x80000000,0x3f800000,0xbf800000,0x7f800000,0xff800000,0x7fc12345,0xffc54321,0x7fa00001,0xffa12345,1,0x7f7fffff,0x41400000,0x437e0000,0x42c80000,0x4b189680,0x4ba83308,0x3f9e0652,0xc2f6e979,0x3eaaaaab,0x00800001,0x4b000001};
    puts("# actual x86 SSE scalar instructions; masked nearest MXCSR; execution host/translation must be recorded separately");
    for(unsigned op=0;op<3;op++)for(unsigned a=0;a<22;a++)for(unsigned b=0;b<22;b++)printf("Q %u %u %u -> %u\n",op,v[a],v[b],scalar(v[a],v[b],op));
    __asm__ volatile("ldmxcsr %0"::"m"(saved));
    return 0;
}
