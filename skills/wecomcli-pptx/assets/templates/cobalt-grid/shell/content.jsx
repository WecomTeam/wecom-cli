<Slide style={{ background: '#F0EBDE', padding: '19px 48px 14px 48px', position: 'relative' }}>
    <svg width={1280} height={719} viewBox='0 0 1280 719' style={{ position: 'absolute', top: 1, left: 0, zIndex: 0 }}>
        <defs>
            <pattern id='grid' width='32' height='32' patternUnits='userSpaceOnUse'>
                <path d='M 32 0 L 0 0 0 32' fill='none' stroke='#DBD8DE' strokeWidth='1' />
            </pattern>
        </defs>
        <rect x='0' y='0' width='1280' height='719' fill='#F0EBDE' />
        <rect x='0' y='0' width='1280' height='719' fill='url(#grid)' />
    </svg>
    <Box style={{ position: 'relative', zIndex: 1, width: 1184, height: 687 }}>
        <Box style={{ width: 1184, height: 1.5, background: '#1F2BE0' }} />
        <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-end', marginTop: 16, height: 127.5, paddingBottom: 14, borderBottom: '1.5px solid #1F2BE0' }}>
            <Text style={{ width: 940, fontFamily: 'STSong', fontSize: 34, fontWeight: 700, color: '#1F2BE0', lineHeight: 1.3 }}>〔标题〕</Text>
            <Text style={{ fontFamily: 'Courier New', fontSize: 13, color: '#1F2BE0', letterSpacing: 1.5, whiteSpace: 'nowrap', paddingBottom: 6 }}>〔LAB TAG〕</Text>
        </Box>
        <Box style={{ width: 1184, height: 436, marginTop: 24 }}>
            〔B区内容〕
        </Box>
        <Text style={{ position: 'absolute', left: 0, bottom: 26, width: 960, fontFamily: 'Microsoft YaHei', fontSize: 13, color: '#1F2BE0', lineHeight: 1.5 }}>〔来源〕<br />〔注〕</Text>
        <Text style={{ position: 'absolute', right: 0, bottom: 30, fontFamily: 'Courier New', fontSize: 13, color: '#1F2BE0', letterSpacing: 1 }}>〔NN / 总页〕</Text>
        <Box style={{ position: 'absolute', left: 0, bottom: 0, width: 1184, height: 1.5, background: '#1F2BE0' }} />
    </Box>
</Slide>
