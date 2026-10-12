#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""生成知歇路演 PPT 深色导航版 29 个 .slide 源。
用法: python3 gen_dark_slides.py   (在本目录运行, 输出到 ../slides/)
"""
import os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'slides')

BG = '#0B1220'
PANEL = '#1B2740'
ACC = '#4FE0B0'
TXT = '#E8EEF7'
BORDER = 'rgba(232,238,247,0.12)'
LINE = 'rgba(232,238,247,0.10)'
HL_BG = 'rgba(79,224,176,0.10)'
HL_BD = 'rgba(79,224,176,0.4)'
PH_BG = 'rgba(79,224,176,0.06)'
PH_BD = 'rgba(79,224,176,0.45)'

CHAPTERS = ['项目背景', '目标用户群体', '应用场景', '智能体开发过程',
            '实施计划', '预期成果', '创新点', '应用价值']
PAGE_RANGE = ['P03–P05', 'P06–P08', 'P09–P11', 'P12–P16',
              'P17–P21', 'P22–P23', 'P24–P26', 'P27–P28']

LOGO = ("<svg width={24} height={24} viewBox='0 0 40 40'>"
        "<path d='M28 10 A17 17 0 1 0 28 30' fill='none' stroke='" + ACC + "' strokeWidth='2.5' />"
        "<rect x='15' y='15' width='2.4' height='10' rx='1.2' fill='" + ACC + "' />"
        "<rect x='20' y='15' width='2.4' height='10' rx='1.2' fill='" + ACC + "' />"
        "</svg>")


def nav_bar(active):
    items = []
    for i, ch in enumerate(CHAPTERS):
        if i == active:
            items.append("<Text style={{ fontSize: 21.33, color: '%s', fontWeight: 'bold' }}>%s</Text>" % (ACC, ch))
        else:
            items.append("<Text style={{ fontSize: 21.33, color: '%s', opacity: 0.5 }}>%s</Text>" % (TXT, ch))
    return ("    <Box style={{ height: 56, flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', borderBottom: '1px solid rgba(232,238,247,0.14)' }}>\n"
            "      <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 10 }}>\n"
            "        " + LOGO + "\n"
            "        <Text style={{ fontSize: 18, fontWeight: 'bold', color: '" + ACC + "' }}>知歇 ZhiXie</Text>\n"
            "      </Box>\n"
            "      <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>\n"
            "        " + "\n        ".join(items) + "\n"
            "      </Box>\n"
            "    </Box>\n")


def title_block(chap_idx, title, no):
    label = '%02d · %s' % (chap_idx + 1, CHAPTERS[chap_idx])
    return ("    <Box style={{ height: 100, flexDirection: 'row', alignItems: 'flex-end', justifyContent: 'space-between', paddingBottom: 14 }}>\n"
            "      <Box style={{ flexDirection: 'column', gap: 4 }}>\n"
            "        <Text style={{ fontSize: 21.33, fontFamily: 'Consolas', color: '" + ACC + "', letterSpacing: 2 }}>" + label + "</Text>\n"
            "        <Text style={{ fontSize: 40, fontWeight: 'bold', color: '" + TXT + "' }}>" + title + "</Text>\n"
            "      </Box>\n"
            "      <Text style={{ fontSize: 20, fontFamily: 'Consolas', color: '" + TXT + "', opacity: 0.35 }}>" + str(no) + "</Text>\n"
            "    </Box>\n")


def footer(no):
    return ("    <Box style={{ height: 40, flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between' }}>\n"
            "      <Text style={{ fontSize: 18, color: '" + TXT + "', opacity: 0.45 }}>知歇 ZhiXie · V1.0.0</Text>\n"
            "      <Text style={{ fontSize: 18, fontFamily: 'Consolas', color: '" + TXT + "', opacity: 0.45 }}>" + str(no) + " / 29</Text>\n"
            "    </Box>\n")


def content_slide(no, chap_idx, title, body):
    return ("<Slide style={{ width: 1280, height: 720, background: '" + BG + "', padding: '16px 64px' }}>\n"
            + nav_bar(chap_idx)
            + title_block(chap_idx, title, no)
            + "    " + body + "\n"
            + footer(no)
            + "</Slide>\n")


def hl_card(width=None, height=None, pad='18px 22px', extra=''):
    s = 'flexDirection: ' + "'column'" if False else ''
    w = " width: %d," % width if width else ''
    h = " height: %d," % height if height else ''
    return "<Box style={{" + w + h + " background: '" + HL_BG + "', border: '1px solid " + HL_BD + "', borderRadius: 12, padding: '" + pad + "'" + (', ' + extra if extra else '') + "}}>"


def card(width=None, height=None, pad='18px 22px', extra=''):
    w = " width: %d," % width if width else ''
    h = " height: %d," % height if height else ''
    return "<Box style={{" + w + h + " background: '" + PANEL + "', border: '1px solid " + BORDER + "', borderRadius: 12, padding: '" + pad + "'" + (', ' + extra if extra else '') + "}}>"


def ph_box(width=None, height=None, num='', name='', suggest='', note=False):
    w = "width: %d, " % width if width else ''
    h = "height: %d, " % height if height else ''
    n = ("<Text style={{ fontSize: 22, fontFamily: 'Consolas', color: '" + ACC + "', letterSpacing: 1 }}>" + num + "</Text>\n        ")
    nm = "<Text style={{ fontSize: 28, fontWeight: 'bold', color: '" + TXT + "', textAlign: 'center' }}>" + name + "</Text>\n        "
    sg = "<Text style={{ fontSize: 22, color: '" + TXT + "', opacity: 0.55, textAlign: 'center', lineHeight: 1.6 }}>" + suggest + "</Text>"
    nt = ("\n        <Text style={{ fontSize: 22, color: '" + ACC + "', textAlign: 'center', lineHeight: 1.5 }}>※ 建议补充真实调研数据，不编造</Text>") if note else ''
    return ("<Box style={{ " + w + h + "flexDirection: 'column', justifyContent: 'center', alignItems: 'center', gap: 10, background: '" + PH_BG + "', border: '2px dashed " + PH_BD + "', borderRadius: 12, padding: '0 20px' }}>\n"
            "        " + n + nm + sg + nt + "\n"
            "      </Box>")


# ---------------- P01 封面 ----------------
P01 = """<Slide style={{ width: 1280, height: 720, background: '%s', padding: '64px 80px', flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between' }}>
    <Box style={{ width: 660, flexDirection: 'column', gap: 20 }}>
      <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>
        <svg width={44} height={44} viewBox='0 0 40 40'>
          <circle cx='20' cy='20' r='17' fill='none' stroke='%s' strokeWidth='1' opacity='0.4' />
          <path d='M28 10 A17 17 0 1 0 28 30' fill='none' stroke='%s' strokeWidth='2.5' />
          <rect x='15' y='15' width='2.4' height='10' rx='1.2' fill='%s' />
          <rect x='20' y='15' width='2.4' height='10' rx='1.2' fill='%s' />
        </svg>
        <Text style={{ fontSize: 22, color: '%s', opacity: 0.7, letterSpacing: 6 }}>Z H I X I E</Text>
      </Box>
      <Text style={{ fontSize: 76, fontWeight: 'bold', color: '%s', lineHeight: 1.15 }}>知歇 <span style={{ color: '%s' }}>ZhiXie</span></Text>
      <Text style={{ fontSize: 30, color: '%s', letterSpacing: 3 }}>知时而歇，张弛有度</Text>
      <Box style={{ height: 3, width: 120, background: 'linear-gradient(135deg, #4FE0B0 0%%, #2E9E8F 100%%)', borderRadius: 2 }} />
      <Text style={{ fontSize: 28, color: '%s', opacity: 0.85, lineHeight: 1.55 }}>会听人话的桌面休息提醒。<br />说一句话，它就把提醒设好。</Text>
      <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 12, marginTop: 8 }}>
        <svg width={26} height={26} viewBox='0 0 22 22'>
          <circle cx='11' cy='11' r='10' fill='rgba(79,224,176,0.12)' stroke='%s' strokeWidth='1.5' />
          <path d='M6 11 L9.5 14.5 L16 8' fill='none' stroke='%s' strokeWidth='2' />
        </svg>
        <Text style={{ fontSize: 26, color: '%s' }}>已经做完 · 双平台可直接下载安装</Text>
      </Box>
      <Text style={{ fontSize: 20, color: '%s', fontFamily: 'Consolas', marginTop: 6 }}>https://smileher.github.io/ZhiXie/</Text>
      <Text style={{ fontSize: 22, color: '%s', opacity: 0.5, letterSpacing: 1, marginTop: 8 }}>创新主赛道 · AIGC 创新组</Text>
    </Box>
    <Box style={{ width: 420, height: 420, justifyContent: 'center', alignItems: 'center' }}>
      <svg width={420} height={420} viewBox='0 0 440 440'>
        <circle cx='220' cy='220' r='200' fill='rgba(79,224,176,0.05)' />
        <circle cx='220' cy='220' r='200' fill='none' stroke='%s' strokeWidth='1' opacity='0.22' />
        <circle cx='220' cy='220' r='146' fill='none' stroke='%s' strokeWidth='1' opacity='0.32' />
        <circle cx='220' cy='220' r='92' fill='none' stroke='%s' strokeWidth='1.5' opacity='0.5' />
        <circle cx='220' cy='220' r='5' fill='%s' />
        <path d='M220 220 L392 112' fill='none' stroke='%s' strokeWidth='2' opacity='0.7' />
        <path d='M330 128 A200 200 0 0 1 392 214' fill='none' stroke='%s' strokeWidth='2.5' opacity='0.9' />
        <circle cx='392' cy='112' r='4' fill='%s' />
      </svg>
    </Box>
</Slide>
""" % (BG, ACC, ACC, ACC, ACC, ACC, TXT, ACC, ACC, ACC, ACC, ACC, ACC, ACC, ACC, ACC, ACC, ACC, ACC, ACC, ACC, ACC)

# ---------------- P02 目录 ----------------
rows = []
for i, ch in enumerate(CHAPTERS):
    bd = '' if i == 7 else " borderBottom: '1px solid " + LINE + "',"
    rows.append("      <Box style={{ height: 54, flexDirection: 'row', alignItems: 'center', gap: 18," + bd + " }}>\n"
                "        <Text style={{ fontSize: 30, fontFamily: 'Consolas', color: '" + ACC + "', width: 56 }}>%02d</Text>\n"
                "        <Text style={{ fontSize: 26, color: '%s', flex: 1 }}>%s</Text>\n"
                "        <Text style={{ fontSize: 20, fontFamily: 'Consolas', color: '%s', opacity: 0.45 }}>%s</Text>\n"
                "      </Box>" % (i + 1, TXT, ch, TXT, PAGE_RANGE[i]))
P02 = """<Slide style={{ width: 1280, height: 720, background: '%s', padding: '0 80px', flexDirection: 'row', alignItems: 'center', gap: 60 }}>
    <Box style={{ width: 340, flexDirection: 'column', gap: 8 }}>
      <Text style={{ fontSize: 56, fontWeight: 'bold', color: '%s' }}>Contents</Text>
      <Text style={{ fontSize: 40, fontWeight: 'bold', color: '%s' }}>// 目录</Text>
      <Box style={{ height: 3, width: 120, background: 'linear-gradient(135deg, #4FE0B0 0%%, #2E9E8F 100%%)', borderRadius: 2, marginTop: 10 }} />
      <svg width={200} height={140} viewBox='0 0 220 150' style={{ marginTop: 24 }}>
        <circle cx='110' cy='75' r='66' fill='none' stroke='%s' strokeWidth='1' opacity='0.3' />
        <circle cx='110' cy='75' r='40' fill='none' stroke='%s' strokeWidth='1.5' opacity='0.45' />
        <path d='M110 75 L164 44' fill='none' stroke='%s' strokeWidth='2' opacity='0.7' />
        <path d='M146 51 A66 66 0 0 1 164 84' fill='none' stroke='%s' strokeWidth='2.5' />
        <circle cx='110' cy='75' r='4' fill='%s' />
      </svg>
      <Text style={{ fontSize: 20, color: '%s', opacity: 0.5, letterSpacing: 1, marginTop: 20 }}>创新主赛道 · AIGC 创新组</Text>
    </Box>
    <Box style={{ flex: 1, flexDirection: 'column' }}>
%s
    </Box>
</Slide>
""" % (BG, TXT, ACC, ACC, ACC, ACC, ACC, ACC, ACC, '\n'.join(rows))

# ---------------- P03 问题引入 ----------------
sig = []
signals = [('1', '眼睛先报警', '干涩、发胀、看东西发花'),
           ('2', '肩颈变僵硬', '一动脖子就咔咔响'),
           ('3', '腰背会发麻', '起身才知道坐了多久')]
for n, t, d in signals:
    sig.append("      " + card(height=130, pad='0 24px') + "\n"
               "        <Text style={{ fontSize: 40, fontFamily: 'Consolas', color: '" + ACC + "' }}>%s</Text>\n"
               "        <Box style={{ flex: 1, flexDirection: 'column', gap: 6 }}>\n"
               "          <Text style={{ fontSize: 28, fontWeight: 'bold', color: '%s' }}>%s</Text>\n"
               "          <Text style={{ fontSize: 24, color: '%s', opacity: 0.65 }}>%s</Text>\n"
               "        </Box>\n"
               "      </Box>" % (n, TXT, t, TXT, d))
P03_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 40, alignItems: 'center' }}>\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 18 }}>\n"
            "        <Text style={{ fontSize: 36, fontWeight: 'bold', color: '" + ACC + "', lineHeight: 1.4 }}>您上一次连续坐在电脑前，<br />是多久？</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>一进入专注，时间感就没了。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>身体一直在发信号，只是被忽略。</Text>\n"
            "        " + hl_card(pad='18px 22px') + "\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', lineHeight: 1.5 }}>谁能用上：<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>学生、久坐办公的人</span></Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "      <Box style={{ width: 520, flexDirection: 'column', gap: 20 }}>\n"
            + "\n".join(sig) + "\n"
            "      </Box>\n"
            "    </Box>")

# ---------------- P04 现有方案不足 ----------------
alts = [('1', '手机闹钟', '到点就响吓一跳；专注时被打断，还得解锁去操作'),
        ('2', '日历 / 待办', '字段要一个个填；当时没设，过后就想不起来'),
        ('3', '同类休息软件', '多是英文界面、一堆表单，有的还催你打卡比拼')]
altr = []
for n, t, d in alts:
    altr.append("      " + card(height=116, pad='16px 22px') + "\n"
                "        <Box style={{ flexDirection: 'column', gap: 6 }}>\n"
                "          <Box style={{ flexDirection: 'row', alignItems: 'baseline', gap: 12 }}>\n"
                "            <Text style={{ fontSize: 32, fontFamily: 'Consolas', color: '" + ACC + "' }}>%s</Text>\n"
                "            <Text style={{ fontSize: 26, fontWeight: 'bold', color: '%s' }}>%s</Text>\n"
                "          </Box>\n"
                "          <Text style={{ fontSize: 22, color: '%s', opacity: 0.75, lineHeight: 1.4 }}>%s</Text>\n"
                "        </Box>\n"
                "      </Box>" % (n, TXT, t, TXT, d))
P04_BODY = ("<Box style={{ height: 470, flexDirection: 'column', gap: 10 }}>\n"
            + "\n".join(altr) + "\n"
            "      " + hl_card(pad='20px 24px') + "\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', lineHeight: 1.5 }}>真正缺的，是一个<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>听得懂人话、到点温柔提醒</span>的桌面工具。</Text>\n"
            "        </Box>\n"
            "    </Box>")

# ---------------- P05 知歇是什么 ----------------
P05_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 40, alignItems: 'center' }}>\n"
            "      <Box style={{ width: 500, flexDirection: 'column', gap: 20 }}>\n"
            "        " + ph_box(height=215, num='截图 ③', name='知歇主界面', suggest='建议画面：休息节律页或智能体页首屏') + "\n"
            "        " + ph_box(height=215, num='截图 ④', name='一句话建提醒', suggest='建议画面：输入「每久坐 50 分钟提醒我」<br />的解析结果页') + "\n"
            "      </Box>\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 12 }}>\n"
            "        <Text style={{ fontSize: 36, fontWeight: 'bold', color: '" + ACC + "', lineHeight: 1.3 }}>说一句话，<br />提醒就建好了。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.4 }}>「每久坐 50 分钟提醒我」——不必填表单。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.4 }}>到点弹窗，可推迟，也能顺带锁屏。</Text>\n"
            "        " + card(pad='12px 18px') + "\n"
            "          <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}><span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>智能体</span>（软件里那个帮你把话变成提醒的小助手）帮你听懂这句话。</Text>\n"
            "        </Box>\n"
            "        " + hl_card(pad='12px 18px') + "\n"
            "          <Text style={{ fontSize: 26, color: '" + TXT + "', lineHeight: 1.5 }}>状态：<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>已经做完 · 双平台可下载安装</span></Text>\n"
            "          <Text style={{ fontSize: 20, color: '" + ACC + "', fontFamily: 'Consolas', marginTop: 6 }}>https://smileher.github.io/ZhiXie/</Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "    </Box>")

# ---------------- P06/07/08 用户 ----------------
def user_body(num, phname, phsuggest, headline, l1, l2, typ, key):
    return ("<Box style={{ height: 470, flexDirection: 'row', gap: 40, alignItems: 'center' }}>\n"
            "      " + ph_box(width=540, height=440, num=num, name=phname, suggest=phsuggest, note=True) + "\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 18 }}>\n"
            "        <Text style={{ fontSize: 36, fontWeight: 'bold', color: '" + ACC + "', lineHeight: 1.4 }}>" + headline + "</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>" + l1 + "</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>" + l2 + "</Text>\n"
            "        " + hl_card(pad='18px 22px') + "\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', lineHeight: 1.5 }}>" + typ + "：<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>" + key + "</span></Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "    </Box>")

def _bar_row(label, val_text, bar_w, track_w=230, label_w=140):
    return ("        <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 10, height: 30 }}>\n"
            "          <Text style={{ fontSize: 20, color: '" + TXT + "', opacity: 0.75, width: '" + str(label_w) + "' }}>" + label + "</Text>\n"
            "          <Box style={{ width: " + str(track_w) + ", height: 22, borderRadius: 6, background: 'rgba(232,238,247,0.08)' }}>\n"
            "            <Box style={{ width: " + str(bar_w) + ", height: 22, borderRadius: 6, background: 'linear-gradient(90deg, #4FE0B0, #2E9E8F)' }} />\n"
            "          </Box>\n"
            "          <Text style={{ fontSize: 19, fontFamily: 'Consolas', color: '" + ACC + "', fontWeight: 'bold' }}>" + val_text + "</Text>\n"
            "        </Box>")

P06_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 40, alignItems: 'center' }}>\n"
            "      <Box style={{ width: 540, height: 440, background: '" + PANEL + "', border: '1px solid " + BORDER + "', borderRadius: 14, padding: '24px 24px', flexDirection: 'column', gap: 14 }}>\n"
            "        <Text style={{ fontSize: 24, fontWeight: 'bold', color: '" + TXT + "' }}>高等教育在学规模 · 2024</Text>\n"
            + _bar_row('在学总规模', '4846万', 230) + "\n"
            + _bar_row('本专科在校', '3891万', 185) + "\n"
            + _bar_row('在学研究生', '410万', 22) + "\n"
            "        <Box style={{ height: 1, background: '" + LINE + "' }} />\n"
            "        <Text style={{ fontSize: 22, color: '" + TXT + "' }}>毛入学率 <span style={{ fontSize: 30, color: '" + ACC + "', fontWeight: 'bold' }}>60.8%</span>，进入普及化阶段</Text>\n"
            "        <Text style={{ fontSize: 19, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>2025 备考人群：考研报名 <span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>388 万</span><br />国考报名 <span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>341.6 万</span>，人群持续扩大</Text>\n"
            "        <Text style={{ fontSize: 15, color: '" + TXT + "', opacity: 0.5 }}>数据来源：教育部《2024 年全国教育事业发展统计公报》</Text>\n"
            "      </Box>\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 18 }}>\n"
            "        <Text style={{ fontSize: 36, fontWeight: 'bold', color: '" + ACC + "', lineHeight: 1.4 }}>最容易坐忘的，<br />是在校生。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>备考、写论文、做实验，一坐一整天。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>有手机有电脑，却少一个提醒自己休息的。</Text>\n"
            "        " + hl_card(pad='18px 22px') + "\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', lineHeight: 1.5 }}>典型：<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>大学生、考研 / 考公族</span></Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "    </Box>")
P07_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 40, alignItems: 'center' }}>\n"
            "      <Box style={{ width: 540, height: 440, background: '" + PANEL + "', border: '1px solid " + BORDER + "', borderRadius: 14, padding: '24px 24px', flexDirection: 'column', gap: 12 }}>\n"
            "        <Text style={{ fontSize: 24, fontWeight: 'bold', color: '" + TXT + "' }}>职场久坐 · 关键数据</Text>\n"
            + _bar_row('日均久坐超 6 小时', '28.8%', 73, 180, 170) + "\n"
            + _bar_row('单次久坐 2 小时+', '71.4%', 180, 180, 170) + "\n"
            + _bar_row('视力疲劳', '53.1%', 134, 180, 170) + "\n"
            + _bar_row('颈椎腰椎不适', '48.9%', 123, 180, 170) + "\n"
            "        <Box style={{ height: 1, background: '" + LINE + "' }} />\n"
            "        <Text style={{ fontSize: 22, color: '" + TXT + "', lineHeight: 1.5 }}>每日久坐超过 8 小时，心血管事件风险<br />升高约 <span style={{ fontSize: 28, color: '" + ACC + "', fontWeight: 'bold' }}>86%</span></Text>\n"
            "        <Text style={{ fontSize: 15, color: '" + TXT + "', opacity: 0.5 }}>数据来源：中国居民慢性病与营养监测 · 智联招聘职场调研</Text>\n"
            "      </Box>\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 18 }}>\n"
            "        <Text style={{ fontSize: 36, fontWeight: 'bold', color: '" + ACC + "', lineHeight: 1.4 }}>久坐办公族，<br />负担最重。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>视疲劳、肩颈问题，常年排在前面。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>他们要的是不打断、又能见效的提醒。</Text>\n"
            "        " + hl_card(pad='18px 22px') + "\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', lineHeight: 1.5 }}>典型：<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>程序、文案、设计、运营</span></Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "    </Box>")
P08_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 40, alignItems: 'center' }}>\n"
            "      <Box style={{ width: 540, height: 440, background: '" + PANEL + "', border: '1px solid " + BORDER + "', borderRadius: 14, padding: '24px 24px', flexDirection: 'column', gap: 12 }}>\n"
            "        <Text style={{ fontSize: 24, fontWeight: 'bold', color: '" + TXT + "' }}>青年久坐应对意愿 · 公开调研</Text>\n"
            + _bar_row('尝试过「轻养生」', '97.2%', 180, 180, 190) + "\n"
            + _bar_row('用设备 / 站立办公', '53.5%', 99, 180, 190) + "\n"
            + _bar_row('短暂起身休息交流', '45.4%', 84, 180, 190) + "\n"
            + _bar_row('定时提醒强制起身', '35.9%', 66, 180, 190) + "\n"
            "        <Box style={{ height: 1, background: '" + LINE + "' }} />\n"
            "        <Text style={{ fontSize: 15, color: '" + TXT + "', opacity: 0.5 }}>数据来源：中国青年报社社会调查中心联合问卷网<br />（1334 名受访青年，新华网发布）</Text>\n"
            "      </Box>\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 18 }}>\n"
            "        <Text style={{ fontSize: 36, fontWeight: 'bold', color: '" + ACC + "', lineHeight: 1.4 }}>需求是真的，<br />不是我们猜的。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>面对久坐，青年们已经在主动找办法。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>只是现有办法都靠自觉，坚持不下来。</Text>\n"
            "        " + hl_card(pad='18px 22px') + "\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', lineHeight: 1.5 }}><span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>35.9%</span> 明确愿意「设置定时提醒，<br />强制起身活动、远眺」——正是知歇在做的事</Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "    </Box>")

# ---------------- P09/10 场景 ----------------
def scene_body(title, rows_, concl):
    rr = []
    for t, d in rows_:
        rr.append("        " + card(height=130, pad='0 24px') + "\n"
                  "          <Box style={{ flex: 1, flexDirection: 'column', gap: 8 }}>\n"
                  "            <Text style={{ fontSize: 30, fontWeight: 'bold', color: '" + TXT + "' }}>" + t + "</Text>\n"
                  "            <Text style={{ fontSize: 26, color: '" + TXT + "', opacity: 0.7, lineHeight: 1.45 }}>" + d + "</Text>\n"
                  "          </Box>\n"
                  "        </Box>")
    return ("<Box style={{ height: 470, flexDirection: 'row', gap: 32, alignItems: 'center' }}>\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 20 }}>\n"
            + "\n".join(rr) + "\n"
            "      </Box>\n"
            "      " + hl_card(width=320, height=430, pad='30px 26px') + "\n"
            "          <Text style={{ fontSize: 24, color: '" + ACC + "', letterSpacing: 2, textAlign: 'center' }}>共同点</Text>\n"
            "          <Text style={{ fontSize: 32, fontWeight: 'bold', color: '" + TXT + "', textAlign: 'center', lineHeight: 1.5 }}>" + concl + "</Text>\n"
            "        </Box>\n"
            "    </Box>")

P09_BODY = scene_body('学生', [('机房与实验课', '上机一节连一节，下课铃是唯一的提醒'),
                               ('晚自习 / 图书馆', '戴上耳机进入状态，一抬头两小时过去'),
                               ('备考冲刺', '真题一套接一套，离开屏幕才想起歇')],
                      '不是不想歇，<br />是没人提醒')
P10_BODY = scene_body('职场', [('写代码 / 改文档', '进入心流停不下来，一坐一上午'),
                               ('会议连着屏幕', '会议室与工位来回切，眼睛没喘过气'),
                               ('创意设计赶稿', '改到深处，脖子已经前倾很久')],
                      '不打断心流，<br />又能见效')

# ---------------- P11 跨平台 ----------------
P11_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 40, alignItems: 'center' }}>\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 18 }}>\n"
            "        <Text style={{ fontSize: 36, fontWeight: 'bold', color: '" + ACC + "', lineHeight: 1.4 }}>同一套提醒，<br />两套系统都能跑。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>Windows x64 / ARM64 与 macOS 都提供支持。</Text>\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>提醒数据存在本机，不上传、不绑定账号。</Text>\n"
            "        " + hl_card(pad='18px 22px') + "\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', lineHeight: 1.5 }}>隐私立场：<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>数据留在本机，你说了算</span></Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "      " + card(width=560, height=420, pad='30px 30px') + "\n"
            "        <svg width={500} height={260} viewBox='0 0 500 260'>\n"
            "          <rect x='20' y='30' width='200' height='150' rx='10' fill='#0B1220' stroke='rgba(232,238,247,0.2)' strokeWidth='1.5' />\n"
            "          <rect x='20' y='30' width='200' height='28' rx='10' fill='rgba(79,224,176,0.12)' />\n"
            "          <circle cx='38' cy='44' r='4' fill='#4FE0B0' /><circle cx='54' cy='44' r='4' fill='#E8EEF7' />\n"
            "          <text x='120' y='120' fontFamily='微软雅黑' fontSize='22' fill='#E8EEF7' textAnchor='middle'>Windows</text>\n"
            "          <rect x='280' y='30' width='200' height='150' rx='10' fill='#0B1220' stroke='rgba(232,238,247,0.2)' strokeWidth='1.5' />\n"
            "          <rect x='280' y='30' width='200' height='28' rx='10' fill='rgba(79,224,176,0.12)' />\n"
            "          <circle cx='298' cy='44' r='4' fill='#4FE0B0' /><circle cx='314' cy='44' r='4' fill='#E8EEF7' />\n"
            "          <text x='380' y='120' fontFamily='微软雅黑' fontSize='22' fill='#E8EEF7' textAnchor='middle'>macOS</text>\n"
            "          <circle cx='250' cy='105' r='17' fill='rgba(79,224,176,0.2)' stroke='#4FE0B0' strokeWidth='1.5' />\n"
            "          <path d='M242 105 L247 110 L257 99' stroke='#4FE0B0' strokeWidth='2.5' fill='none' />\n"
            "        </svg>\n"
            "        <Text style={{ fontSize: 26, color: '" + TXT + "', opacity: 0.7, textAlign: 'center' }}>双平台安装包，下载即用</Text>\n"
            "      </Box>\n"
            "    </Box>")

# ---------------- P12 人机分工 ----------------
P12_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 28, alignItems: 'stretch' }}>\n"
            "      " + card(pad='26px 28px') + "\n"
            "          <Text style={{ fontSize: 34, fontWeight: 'bold', color: '" + ACC + "' }}>我（开发者）负责</Text>\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.85, lineHeight: 1.55 }}>· 定产品方向与功能边界</Text>\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.85, lineHeight: 1.55 }}>· 设计数据结构和交互</Text>\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.85, lineHeight: 1.55 }}>· 做关键判断与取舍</Text>\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.85, lineHeight: 1.55 }}>· 跑测试、把关质量</Text>\n"
            "        </Box>\n"
            "      " + hl_card(pad='26px 28px') + "\n"
            "          <Text style={{ fontSize: 34, fontWeight: 'bold', color: '" + ACC + "' }}>AI 助手负责</Text>\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.85, lineHeight: 1.55 }}>· 按需求写代码与界面</Text>\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.85, lineHeight: 1.55 }}>· 起草说明文档与文案</Text>\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.85, lineHeight: 1.55 }}>· 查资料、给实现建议</Text>\n"
            "          <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.85, lineHeight: 1.55 }}>· 出初稿，我来审改</Text>\n"
            "        </Box>\n"
            "    </Box>")

# ---------------- P13 提示词与选型 ----------------
P13_BODY = ("<Box style={{ height: 470, flexDirection: 'column', gap: 16 }}>\n"
            "      <Box style={{ height: 200, flexDirection: 'row', gap: 16, alignItems: 'stretch' }}>\n"
            "        " + card(pad='20px 24px') + "\n"
            "          <Text style={{ fontSize: 26, fontWeight: 'bold', color: '" + ACC + "' }}>提示词（约束格式）</Text>\n"
            "          <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>\u201c只输出 JSON：type / time / repeat / action，不要解释。\u201d</Text>\n"
            "        </Box>\n"
            "        <Box style={{ width: 60, justifyContent: 'center', alignItems: 'center' }}>\n"
            "          <Text style={{ fontSize: 40, color: '" + ACC + "', fontWeight: 'bold' }}>→</Text>\n"
            "        </Box>\n"
            "        " + hl_card(pad='20px 24px') + "\n"
            "          <Text style={{ fontSize: 26, fontWeight: 'bold', color: '" + ACC + "' }}>结构化结果（JSON）</Text>\n"
            "          <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>{ '{ type:\"rest\", time:\"50m\", repeat:\"daily\", action:\"notify\" }' }</Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "      <Box style={{ height: 254, flexDirection: 'row', gap: 20, alignItems: 'stretch' }}>\n"
            "        " + card(pad='24px 28px') + "\n"
            "          <Text style={{ fontSize: 26, fontWeight: 'bold', color: '" + ACC + "' }}>模型配置（三格，存本机）</Text>\n"
            "          <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', padding: '12px 0', borderBottom: '1px solid " + LINE + "' }}>\n"
            "            <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.6 }}>服务商</Text>\n"
            "            <Text style={{ fontSize: 27, fontWeight: 'bold', color: '" + TXT + "' }}>DeepSeek（可改）</Text>\n"
            "          </Box>\n"
            "          <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', padding: '12px 0', borderBottom: '1px solid " + LINE + "' }}>\n"
            "            <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.6 }}>接口地址</Text>\n"
            "            <Text style={{ fontSize: 25, color: '" + TXT + "' }}>/v1/chat/completions</Text>\n"
            "          </Box>\n"
            "          <Box style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', padding: '12px 0' }}>\n"
            "            <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.6 }}>密钥</Text>\n"
            "            <Text style={{ fontSize: 25, color: '" + TXT + "' }}>sk-****（本地保存）</Text>\n"
            "          </Box>\n"
            "        </Box>\n"
            "        " + hl_card(width=330, height=254, pad='26px 26px') + "\n"
            "          <Text style={{ fontSize: 30, fontWeight: 'bold', color: '" + TXT + "', lineHeight: 1.45 }}>换模型 =<br /><span style={{ color: '" + ACC + "' }}>改配置，不重写</span></Text>\n"
            "          <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.7, lineHeight: 1.5 }}>DeepSeek / 通义 / 本地模型，改两格就能换。</Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "    </Box>")

# ---------------- P14 双通道架构 ----------------
P14_BODY = ("<Box style={{ height: 470, flexDirection: 'column', gap: 14, alignItems: 'center', justifyContent: 'center' }}>\n"
            "      <svg width={1112} height={260} viewBox='0 0 1112 260'>\n"
            "        <rect x='20' y='98' width='180' height='64' rx='10' fill='#1B2740' stroke='rgba(232,238,247,0.2)' strokeWidth='1.5' />\n"
            "        <text x='110' y='138' fontFamily='微软雅黑' fontSize='24' fill='#E8EEF7' textAnchor='middle'>你说一句话</text>\n"
            "        <path d='M200 130 L262 130' stroke='#4FE0B0' strokeWidth='2' />\n"
            "        <rect x='272' y='28' width='240' height='64' rx='10' fill='rgba(79,224,176,0.1)' stroke='#4FE0B0' strokeWidth='1.5' />\n"
            "        <text x='392' y='68' fontFamily='微软雅黑' fontSize='24' fill='#E8EEF7' textAnchor='middle'>云端大模型</text>\n"
            "        <rect x='272' y='168' width='240' height='64' rx='10' fill='rgba(79,224,176,0.1)' stroke='#4FE0B0' strokeWidth='1.5' />\n"
            "        <text x='392' y='208' fontFamily='微软雅黑' fontSize='24' fill='#E8EEF7' textAnchor='middle'>本机规则引擎</text>\n"
            "        <path d='M512 60 L606 122' stroke='#4FE0B0' strokeWidth='2' />\n"
            "        <path d='M512 200 L606 138' stroke='#4FE0B0' strokeWidth='2' />\n"
            "        <rect x='616' y='94' width='210' height='72' rx='10' fill='#1B2740' stroke='#4FE0B0' strokeWidth='1.5' />\n"
            "        <text x='721' y='138' fontFamily='微软雅黑' fontSize='23' fill='#E8EEF7' textAnchor='middle'>校验层（白名单）</text>\n"
            "        <path d='M826 130 L894 130' stroke='#4FE0B0' strokeWidth='2' />\n"
            "        <rect x='904' y='94' width='188' height='72' rx='10' fill='rgba(79,224,176,0.2)' stroke='#4FE0B0' strokeWidth='1.5' />\n"
            "        <text x='998' y='138' fontFamily='微软雅黑' fontSize='24' fill='#E8EEF7' textAnchor='middle'>提醒列表</text>\n"
            "      </svg>\n"
            "      <Box style={{ flexDirection: 'row', gap: 16, width: 1112, alignItems: 'center' }}>\n"
            "        " + card(height=100, pad='12px 18px', extra="alignItems: 'center'") + "\n"
            "            <Text style={{ fontSize: 21, color: '" + TXT + "', lineHeight: 1.3 }}><span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>通道一 · 联网</span>　交给云端大模型，听得懂更自由的说法，只提建议。</Text>\n"
            "        </Box>\n"
            "        " + card(height=100, pad='12px 18px', extra="alignItems: 'center'") + "\n"
            "            <Text style={{ fontSize: 21, color: '" + TXT + "', lineHeight: 1.3 }}><span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>通道二 · 断网</span>　用自带规则引擎，认五类说法，照样听懂。</Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "      " + hl_card(pad='12px 22px') + "\n"
            "          <Text style={{ fontSize: 24, color: '" + TXT + "', lineHeight: 1.35 }}>联网走云端、断网走本机——<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>两条通道，同一道校验</span></Text>\n"
            "        </Box>\n"
            "    </Box>")

# ---------------- P15 校验层 ----------------
checks = [('类型', '只收 rest / notify 等白名单'),
          ('时间', '必须能解析为具体时刻'),
          ('重复', '限定在枚举值内'),
          ('系统操作', '锁屏 / 关机走白名单'),
          ('越界', '一律丢弃，不写进列表')]
crows = []
for k, v in checks:
    crows.append("          <Box style={{ height: 56, flexDirection: 'row', alignItems: 'center', padding: '0 20px', background: '#1B2740', borderBottom: '1px solid " + LINE + "' }}>\n"
                 "            <Text style={{ fontSize: 27, color: '" + TXT + "', width: 220 }}>" + k + "</Text>\n"
                 "            <Text style={{ fontSize: 27, color: '" + TXT + "', opacity: 0.8, flex: 1 }}>" + v + "</Text>\n"
                 "          </Box>")
P15_BODY = ("<Box style={{ height: 470, flexDirection: 'column', gap: 14, alignItems: 'center', justifyContent: 'center' }}>\n"
            "      <Box style={{ width: 1112, flexDirection: 'column' }}>\n"
            "        <Box style={{ height: 56, flexDirection: 'row', alignItems: 'center', padding: '0 20px', background: 'rgba(79,224,176,0.12)', borderRadius: '10px 10px 0 0' }}>\n"
            "          <Text style={{ fontSize: 28, fontWeight: 'bold', color: '" + TXT + "', width: 220 }}>检查项</Text>\n"
            "          <Text style={{ fontSize: 28, fontWeight: 'bold', color: '" + TXT + "', flex: 1 }}>规则</Text>\n"
            "        </Box>\n"
            + "\n".join(crows) + "\n"
            "      </Box>\n"
            "      " + hl_card(pad='16px 24px') + "\n"
            "          <Text style={{ fontSize: 27, color: '" + TXT + "', lineHeight: 1.45 }}>AI 说的先核一遍：<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>不合规的进不了你的提醒列表</span></Text>\n"
            "        </Box>\n"
            "    </Box>")

# ---------------- P16 调试实录 ----------------
steps = [('1', '案例', '「三十分钟后锁屏」被理解成次日 09:00'),
         ('2', '根因', '旧代码只认阿拉伯数字，「三十」取不到'),
         ('3', '修法', '按「数量词 + 单位 + 后」扫描，中文数字也能认'),
         ('4', '兜底', '每次改动自动跑 <span>138 项</span>测试，全绿才算完成')]
srows = []
for n, t, d in steps:
    hl = " background: '" + HL_BG + "', border: '1px solid " + HL_BD + "'," if n == '4' else " background: '" + PANEL + "', border: '1px solid " + BORDER + "',"
    srows.append("        <Box style={{ height: 96, flexDirection: 'row', alignItems: 'center', gap: 18, padding: '0 22px'," + hl + " borderRadius: 12 }}>\n"
                 "          <Text style={{ fontSize: 38, fontWeight: 'bold', fontFamily: 'Consolas', color: '" + ACC + "' }}>" + n + "</Text>\n"
                 "          <Box style={{ flexDirection: 'column', gap: 4, flex: 1 }}>\n"
                 "            <Text style={{ fontSize: 24, color: '" + ACC + "', letterSpacing: 1 }}>" + t + "</Text>\n"
                 "            <Text style={{ fontSize: 26, color: '" + TXT + "', opacity: 0.85, lineHeight: 1.4 }}>" + d + "</Text>\n"
                 "          </Box>\n"
                 "        </Box>")
P16_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 36, alignItems: 'center' }}>\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 14 }}>\n"
            + "\n".join(srows) + "\n"
            "      </Box>\n"
            "      <Box style={{ width: 440, flexDirection: 'column', gap: 16 }}>\n"
            "        " + ph_box(height=212, num='截图 ⑲', name='修复前后对比', suggest='建议画面：「次日 09:00」与<br />「30 分钟后」并列') + "\n"
            "        " + ph_box(height=212, num='截图 ⑱', name='测试通过截图', suggest='建议画面：pnpm test / cargo test<br />全绿结果或 CI 页面') + "\n"
            "      </Box>\n"
            "    </Box>")

# ---------------- P17 时间轴 ----------------
nodes = [('1', '需求分析', '第 1 步', '用户痛点与场景调研', False),
         ('2', '原型设计', '第 2 步', '信息架构与交互稿', False),
         ('3', '工程实现', '第 3 步', '双平台 + 智能体', False),
         ('4', '测试验证', '第 4 步', '138 项自动化测试', False),
         ('5', '发布上线', '第 5 步', '官网 + 安装包交付', True)]
ncols = []
for n, t, d, desc, last in nodes:
    circ = ("<Box style={{ width: 56, height: 56, borderRadius: 28, backgroundColor: 'rgba(79,224,176,0.3)', border: '1px solid #4FE0B0', justifyContent: 'center', alignItems: 'center' }}>"
            "<Text style={{ fontSize: 28, fontWeight: 'bold', color: '" + ACC + "', fontFamily: 'Consolas' }}>" + n + "</Text></Box>") if last else (
           "<Box style={{ width: 56, height: 56, borderRadius: 28, backgroundColor: '#4FE0B0', justifyContent: 'center', alignItems: 'center' }}>"
           "<Text style={{ fontSize: 28, fontWeight: 'bold', color: '#0B1220', fontFamily: 'Consolas' }}>" + n + "</Text></Box>")
    ncols.append("        <Box style={{ flex: 1, flexDirection: 'column', alignItems: 'center', gap: 12 }}>\n"
                 "          " + circ + "\n"
                 "          <Text style={{ fontSize: 30, fontWeight: 'bold', color: '" + TXT + "' }}>" + t + "</Text>\n"
                 "          <Text style={{ fontSize: 26, color: '" + ACC + "' }}>" + d + "</Text>\n"
                 "          <Text style={{ fontSize: 22, color: '" + TXT + "', opacity: 0.7 }}>" + desc + "</Text>\n"
                 "        </Box>")
P17_BODY = ("<Box style={{ height: 470, flexDirection: 'column', justifyContent: 'center', gap: 28 }}>\n"
            "      <Box style={{ height: 4, width: 1112, background: 'rgba(79,224,176,0.3)', borderRadius: 2 }} />\n"
            "      <Box style={{ flexDirection: 'row', gap: 18, alignItems: 'flex-start' }}>\n"
            + "\n".join(ncols) + "\n"
            "      </Box>\n"
            "      " + hl_card(pad='16px 24px') + "\n"
            "          <Text style={{ fontSize: 27, color: '" + TXT + "', lineHeight: 1.45 }}>软件开发的完整过程——<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>需求、设计、实现、测试、发布，一步不落</span></Text>\n"
            "        </Box>\n"
            "    </Box>")

# ---------------- P18/19 阶段 ----------------
def stage_body(items, badge, badge_desc, done):
    rows_ = []
    for it in items:
        rows_.append("        " + card(height=78, pad='0 24px') + "\n"
                     "          <Text style={{ fontSize: 30, fontWeight: 'bold', color: '" + ACC + "' }}>✓</Text>\n"
                     "          <Text style={{ fontSize: 27, color: '" + TXT + "', opacity: 0.85, flex: 1 }}>" + it + "</Text>\n"
                     "        </Box>")
    return ("<Box style={{ height: 470, flexDirection: 'row', gap: 28, alignItems: 'stretch' }}>\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 14 }}>\n"
            + "\n".join(rows_) + "\n"
            "      </Box>\n"
            "      " + hl_card(width=340, height=446, pad='40px 30px') + "\n"
            "          <Text style={{ fontSize: 52, fontWeight: 'bold', color: '" + ACC + "' }}>" + badge + "</Text>\n"
            "          <Text style={{ fontSize: 26, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.55 }}>" + badge_desc + "</Text>\n"
            "        </Box>\n"
            "    </Box>")

P18_BODY = stage_body(['双平台工程实现（Tauri 2 + Vue 3 + Rust）',
                       '智能体双通道打通：云端大模型 + 本机规则',
                       '138 项自动化测试全绿（77 前端 + 61 Rust）',
                       '休息弹窗 / 自启动 / 开机运行逐项验证',
                       '小范围内测可用，版本 V1.0.0 定稿'],
                      '已完成', '阶段一目标全部达成，进入发布与参赛阶段。', True)
P19_BODY = stage_body(['官网上线，提供下载与介绍',
                       'CI 自动打包：Windows 便携 exe + macOS dmg',
                       '参赛材料成体系：计划书 / 说明 / 测试报告',
                       '路演演示与答辩准备',
                       '收集内测反馈，滚动修正'],
                      '进行中', '按实施计划推进中，各节点可控。', False)

# ---------------- P20 质量保障 ----------------
def qa_card(n, t, lines_):
    ls = ''.join("<Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.75, lineHeight: 1.6 }}>· " + x + "</Text>\n            " for x in lines_)
    return ("      " + card(pad='36px 24px') + "\n"
            "          <Box style={{ width: 56, height: 56, borderRadius: 28, backgroundColor: 'rgba(79,224,176,0.12)', justifyContent: 'center', alignItems: 'center' }}>\n"
            "            <Text style={{ fontSize: 28, fontWeight: 'bold', color: '" + ACC + "', fontFamily: 'Consolas' }}>" + n + "</Text>\n"
            "          </Box>\n"
            "          <Text style={{ fontSize: 30, fontWeight: 'bold', color: '" + TXT + "' }}>" + t + "</Text>\n"
            "          " + ls + "        </Box>")

P20_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 18, alignItems: 'stretch' }}>\n"
            + qa_card('1', '自动化测试', ['138 项覆盖核心逻辑', '每次改动自动运行', '全绿才继续往下做']) + "\n"
            + qa_card('2', '双平台 CI', ['代码一提交即打包', 'Windows + macOS 同测', '产物直接可安装']) + "\n"
            + qa_card('3', '真机验证', ['两套系统分别实装', '提醒 / 弹窗 / 自启动', '按清单逐项过']) + "\n"
            "    </Box>")

# ---------------- P21 风险与应对 ----------------
risks = [('安装包构建偶发失败', 'Windows 打包链路长，偶发中断', '自动重试 + 便携包兜底，双保险'),
         ('单人开发节奏', '一个人精力与时间有限', '人机协同 + 138 项测试兜底'),
         ('双平台差异', '两套系统行为不完全一致', '双平台 CI 实测 + 真机逐项验证')]
rrows = []
for t, impact, fix in risks:
    rrows.append("      " + card(height=120, pad='16px 24px') + "\n"
                 "        <Box style={{ flexDirection: 'column', gap: 8 }}>\n"
                 "          <Box style={{ flexDirection: 'row', alignItems: 'baseline', gap: 12 }}>\n"
                 "            <Text style={{ fontSize: 28, fontWeight: 'bold', color: '" + TXT + "' }}>" + t + "</Text>\n"
                 "            <Text style={{ fontSize: 20, color: '" + TXT + "', opacity: 0.55 }}>" + impact + "</Text>\n"
                 "          </Box>\n"
                 "          <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 12 }}>\n"
                 "            <Text style={{ fontSize: 30, color: '" + ACC + "', fontWeight: 'bold' }}>→</Text>\n"
                 "            <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.85, lineHeight: 1.4 }}>" + fix + "</Text>\n"
                 "          </Box>\n"
                 "        </Box>\n"
                 "      </Box>")
P21_BODY = ("<Box style={{ height: 470, flexDirection: 'column', gap: 16, justifyContent: 'center' }}>\n"
            + "\n".join(rrows) + "\n"
            "    </Box>")

# ---------------- P22 巨型138 ----------------
P22_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 40, alignItems: 'center' }}>\n"
            "      <Box style={{ width: 400, flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 12 }}>\n"
            "        <Text style={{ fontSize: 150, fontWeight: 'bold', fontFamily: 'Consolas', color: '" + ACC + "', lineHeight: 1 }}>138</Text>\n"
            "        <Box style={{ height: 4, width: 120, background: 'linear-gradient(135deg, #4FE0B0 0%, #2E9E8F 100%)', borderRadius: 2 }} />\n"
            "        <Text style={{ fontSize: 28, color: '" + TXT + "', opacity: 0.85, textAlign: 'center' }}>项自动化测试 · 全部通过</Text>\n"
            "        <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.65, textAlign: 'center' }}>77 前端 + 61 Rust</Text>\n"
            "      </Box>\n"
            "      <Box style={{ flex: 1, flexDirection: 'column', gap: 14 }}>\n"
            "        " + ph_box(height=360, num='截图 ⑰', name='双平台下载页', suggest='建议画面：Windows 与 macOS<br />两个下载入口同框') + "\n"
            "        <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.6, textAlign: 'center' }}>装上就能用，不用注册、不用配置</Text>\n"
            "      </Box>\n"
            "    </Box>")

# ---------------- P23 四件套 ----------------
kits = [('01', '软件', '双平台安装包<br />装上就能用', False),
        ('02', '官网', '下载与介绍页<br />自动发版', False),
        ('03', '文档', '计划书 / 使用说明<br />测试报告', False),
        ('04', '测试', '138 项自动检查<br />持续全绿', True)]
krows = []
for n, t, d, hl in kits:
    style = (" background: '" + HL_BG + "', border: '1px solid " + HL_BD + "',") if hl else (" background: '" + PANEL + "', border: '1px solid " + BORDER + "',")
    krows.append("      <Box style={{ flex: 1, flexDirection: 'column', alignItems: 'center', gap: 14, padding: '40px 16px'," + style + " borderRadius: 12 }}>\n"
                 "        <Text style={{ fontSize: 44, fontWeight: 'bold', color: '" + ACC + "', fontFamily: 'Consolas' }}>" + n + "</Text>\n"
                 "        <Text style={{ fontSize: 32, fontWeight: 'bold', color: '" + TXT + "' }}>" + t + "</Text>\n"
                 "        <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.72, textAlign: 'center', lineHeight: 1.5 }}>" + d + "</Text>\n"
                 "      </Box>")
P23_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 18, alignItems: 'stretch' }}>\n"
            + "\n".join(krows) + "\n"
            "    </Box>")

# ---------------- P24 创新①② ----------------
P24_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 24, alignItems: 'stretch' }}>\n"
            "      " + hl_card(pad='24px 26px') + "\n"
            "          <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>\n"
            "            <Text style={{ fontSize: 36, fontFamily: 'Consolas', color: '" + ACC + "' }}>①</Text>\n"
            "            <Text style={{ fontSize: 30, fontWeight: 'bold', color: '" + TXT + "' }}>说一句话建提醒</Text>\n"
            "          </Box>\n"
            "          <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.75, lineHeight: 1.5 }}>自然语言替代一堆表单，第一次用就会。</Text>\n"
            "          <Box style={{ flexDirection: 'row', gap: 14, marginTop: 6 }}>\n"
            "            <Box style={{ flex: 1, flexDirection: 'column', gap: 8, padding: '16px', background: 'rgba(79,224,176,0.14)', borderRadius: 10 }}>\n"
            "              <Text style={{ fontSize: 22, fontWeight: 'bold', color: '" + ACC + "' }}>你做的</Text>\n"
            "              <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.9, lineHeight: 1.45 }}>「每久坐 50 分钟提醒我」</Text>\n"
            "            </Box>\n"
            "            <Box style={{ flex: 1, flexDirection: 'column', gap: 8, padding: '16px', background: '#0B1220', borderRadius: 10 }}>\n"
            "              <Text style={{ fontSize: 22, fontWeight: 'bold', color: '" + TXT + "', opacity: 0.6 }}>传统做法</Text>\n"
            "              <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.75, lineHeight: 1.45 }}>时间 / 重复 / 类型 / 操作……逐个填</Text>\n"
            "            </Box>\n"
            "          </Box>\n"
            "          <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.75, lineHeight: 1.5 }}>门槛只有一个：<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>会说中文</span>。</Text>\n"
            "        </Box>\n"
            "      " + card(pad='24px 26px') + "\n"
            "          <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>\n"
            "            <Text style={{ fontSize: 36, fontFamily: 'Consolas', color: '" + ACC + "' }}>②</Text>\n"
            "            <Text style={{ fontSize: 30, fontWeight: 'bold', color: '" + TXT + "' }}>AI 校验机制</Text>\n"
            "          </Box>\n"
            "          <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.75, lineHeight: 1.5 }}>大模型只提建议，软件把关不让胡说写进列表。</Text>\n"
            "          <Box style={{ flexDirection: 'column', gap: 10, marginTop: 6 }}>\n"
            "            <Box style={{ height: 56, flexDirection: 'row', alignItems: 'center', padding: '0 18px', background: '#0B1220', borderRadius: 10 }}>\n"
            "              <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.85 }}>大模型：理解你的话，出一份结构化建议</Text>\n"
            "            </Box>\n"
            "            <Box style={{ height: 56, flexDirection: 'row', alignItems: 'center', padding: '0 18px', background: 'rgba(79,224,176,0.12)', borderRadius: 10 }}>\n"
            "              <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.85 }}>校验层：类型 / 时间 / 操作，白名单才放行</Text>\n"
            "            </Box>\n"
            "            <Box style={{ height: 56, flexDirection: 'row', alignItems: 'center', padding: '0 18px', background: '#0B1220', borderRadius: 10 }}>\n"
            "              <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.85 }}>越界一律丢弃，<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>提醒列表永远干净</span></Text>\n"
            "            </Box>\n"
            "          </Box>\n"
            "        </Box>\n"
            "    </Box>")

# ---------------- P25 创新③④ ----------------
P25_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 24, alignItems: 'stretch' }}>\n"
            "      " + card(pad='24px 26px') + "\n"
            "          <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>\n"
            "            <Text style={{ fontSize: 36, fontFamily: 'Consolas', color: '" + ACC + "' }}>③</Text>\n"
            "            <Text style={{ fontSize: 30, fontWeight: 'bold', color: '" + TXT + "' }}>人机协同开发</Text>\n"
            "          </Box>\n"
            "          <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.75, lineHeight: 1.5 }}>高职生 + AI 助手，138 项检查兜底，做出跨平台软件。</Text>\n"
            "          <Box style={{ flexDirection: 'row', gap: 12, alignItems: 'center', marginTop: 6 }}>\n"
            "            <Box style={{ flex: 1, flexDirection: 'column', gap: 6, padding: '16px', background: '#0B1220', borderRadius: 10, alignItems: 'center' }}>\n"
            "              <Text style={{ fontSize: 26, fontWeight: 'bold', color: '" + TXT + "' }}>我</Text>\n"
            "              <Text style={{ fontSize: 22, color: '" + TXT + "', opacity: 0.7, textAlign: 'center' }}>定方向 · 设计 · 把关</Text>\n"
            "            </Box>\n"
            "            <Text style={{ fontSize: 32, color: '" + ACC + "', fontWeight: 'bold' }}>+</Text>\n"
            "            <Box style={{ flex: 1, flexDirection: 'column', gap: 6, padding: '16px', background: 'rgba(79,224,176,0.12)', borderRadius: 10, alignItems: 'center' }}>\n"
            "              <Text style={{ fontSize: 26, fontWeight: 'bold', color: '" + TXT + "' }}>AI 助手</Text>\n"
            "              <Text style={{ fontSize: 22, color: '" + TXT + "', opacity: 0.7, textAlign: 'center' }}>写代码 · 文案 · 草稿</Text>\n"
            "            </Box>\n"
            "          </Box>\n"
            "          <Box style={{ height: 60, flexDirection: 'row', alignItems: 'center', justifyContent: 'center', gap: 14, background: 'rgba(79,224,176,0.12)', borderRadius: 10, marginTop: 6 }}>\n"
            "            <Text style={{ fontSize: 26, fontWeight: 'bold', color: '" + TXT + "' }}>→ 知歇 ZhiXie</Text>\n"
            "            <Text style={{ fontSize: 23, color: '" + TXT + "', opacity: 0.75 }}>双平台 · 138 项测试兜底</Text>\n"
            "          </Box>\n"
            "        </Box>\n"
            "      " + hl_card(pad='24px 26px') + "\n"
            "          <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>\n"
            "            <Text style={{ fontSize: 36, fontFamily: 'Consolas', color: '" + ACC + "' }}>④</Text>\n"
            "            <Text style={{ fontSize: 30, fontWeight: 'bold', color: '" + TXT + "' }}>立场让你休息</Text>\n"
            "          </Box>\n"
            "          <Text style={{ fontSize: 28, fontWeight: 'bold', color: '" + TXT + "', lineHeight: 1.5 }}>它的立场是让你休息，<br />不是催你多干。</Text>\n"
            "          <Box style={{ flexDirection: 'column', gap: 10, marginTop: 6 }}>\n"
            "            <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.8 }}>· 没有排行榜，不比谁坐得久</Text>\n"
            "            <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.8 }}>· 没有连续打卡，不制造亏欠感</Text>\n"
            "            <Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.8 }}>· 没有「再看一会」，到点就提醒</Text>\n"
            "          </Box>\n"
            "        </Box>\n"
            "    </Box>")

# ---------------- P26 创新⑤ + 收束 ----------------
inno5 = [('1', '说一句话建提醒', '自然语言替代一堆表单'),
         ('2', 'AI 校验机制', '大模型只提建议，软件把关'),
         ('3', '人机协同开发', '高职生 + AI 做出跨平台软件'),
         ('4', '立场让你休息', '不催干活、不比拼、不制造焦虑')]
irows = []
for n, t, d in inno5:
    irows.append("        <Box style={{ height: 54, flexDirection: 'row', alignItems: 'center', gap: 18, padding: '0 20px', background: '" + PANEL + "', border: '1px solid " + BORDER + "', borderRadius: 10 }}>\n"
                 "          <Text style={{ fontSize: 28, fontFamily: 'Consolas', color: '" + ACC + "', width: 40 }}>" + n + "</Text>\n"
                 "          <Text style={{ fontSize: 25, fontWeight: 'bold', color: '" + TXT + "', width: 280 }}>" + t + "</Text>\n"
                 "          <Text style={{ fontSize: 24, color: '" + TXT + "', opacity: 0.7, flex: 1 }}>" + d + "</Text>\n"
                 "        </Box>")
P26_BODY = ("<Box style={{ height: 470, flexDirection: 'column', gap: 16 }}>\n"
            "      " + hl_card(pad='20px 24px') + "\n"
            "          <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>\n"
            "            <Text style={{ fontSize: 36, fontFamily: 'Consolas', color: '" + ACC + "' }}>⑤</Text>\n"
            "            <Text style={{ fontSize: 30, fontWeight: 'bold', color: '" + TXT + "' }}>双通道 + 本机数据</Text>\n"
            "          </Box>\n"
            "          <Text style={{ fontSize: 26, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.5 }}>断网也能用（云端大模型 + 本机规则引擎）；数据留在本机、隐私可控，<span style={{ color: '" + ACC + "', fontWeight: 'bold' }}>两套系统都是同一个知歇</span>。</Text>\n"
            "        </Box>\n"
            "      <Box style={{ flexDirection: 'column', gap: 10 }}>\n"
            + "\n".join(irows) + "\n"
            "      </Box>\n"
            "    </Box>")

# ---------------- P27 三层价值 ----------------
def value_card(t, lines_):
    ls = ''.join("<Text style={{ fontSize: 25, color: '" + TXT + "', opacity: 0.8, lineHeight: 1.65 }}>· " + x + "</Text>\n            " for x in lines_)
    return ("      " + card(pad='36px 24px') + "\n"
            "          <Text style={{ fontSize: 34, fontWeight: 'bold', color: '" + ACC + "' }}>" + t + "</Text>\n"
            "          <Box style={{ height: 3, width: 60, background: 'rgba(79,224,176,0.5)', borderRadius: 2 }} />\n"
            "          " + ls + "        </Box>")

P27_BODY = ("<Box style={{ height: 470, flexDirection: 'row', gap: 20, alignItems: 'stretch' }}>\n"
            + value_card('个人', ['少久坐、护视力', '提醒温和，到点就说', '数据留本机更安心']) + "\n"
            + value_card('教育', ['高职生 + AI 做出真实产品', '人机协同过程完整可复用', '可作编程课与创业课案例']) + "\n"
            + value_card('社会', ['倡导健康用机习惯', '让「该歇了」被大方说出来', '关注久坐人群的休息权']) + "\n"
            "    </Box>")

# ---------------- P28 对比 + 口号 ----------------
P28_BODY = ("<Box style={{ height: 470, flexDirection: 'column', gap: 20, alignItems: 'center', justifyContent: 'center' }}>\n"
            "      <Box style={{ width: 1000, flexDirection: 'column' }}>\n"
            "        <Box style={{ height: 56, flexDirection: 'row', alignItems: 'center', padding: '0 20px', background: 'rgba(79,224,176,0.12)', borderRadius: '10px 10px 0 0' }}>\n"
            "          <Text style={{ fontSize: 27, fontWeight: 'bold', color: '" + TXT + "', width: 200 }}>维度</Text>\n"
            "          <Text style={{ fontSize: 27, fontWeight: 'bold', color: '" + ACC + "', flex: 1 }}>知歇</Text>\n"
            "          <Text style={{ fontSize: 27, fontWeight: 'bold', color: '" + TXT + "', opacity: 0.6, flex: 1 }}>传统做法</Text>\n"
            "        </Box>\n"
            "        <Box style={{ height: 62, flexDirection: 'row', alignItems: 'center', padding: '0 20px', background: '#1B2740', borderBottom: '1px solid " + LINE + "' }}>\n"
            "          <Text style={{ fontSize: 26, color: '" + TXT + "', width: 200 }}>建提醒</Text>\n"
            "          <Text style={{ fontSize: 26, color: '" + ACC + "', flex: 1 }}>说一句话</Text>\n"
            "          <Text style={{ fontSize: 26, color: '" + TXT + "', opacity: 0.7, flex: 1 }}>填表单</Text>\n"
            "        </Box>\n"
            "        <Box style={{ height: 62, flexDirection: 'row', alignItems: 'center', padding: '0 20px', background: '#1B2740', borderBottom: '1px solid " + LINE + "' }}>\n"
            "          <Text style={{ fontSize: 26, color: '" + TXT + "', width: 200 }}>隐私</Text>\n"
            "          <Text style={{ fontSize: 26, color: '" + ACC + "', flex: 1 }}>本机数据</Text>\n"
            "          <Text style={{ fontSize: 26, color: '" + TXT + "', opacity: 0.7, flex: 1 }}>常上云</Text>\n"
            "        </Box>\n"
            "        <Box style={{ height: 62, flexDirection: 'row', alignItems: 'center', padding: '0 20px', background: '#1B2740', borderRadius: '0 0 10px 10px' }}>\n"
            "          <Text style={{ fontSize: 26, color: '" + TXT + "', width: 200 }}>断网</Text>\n"
            "          <Text style={{ fontSize: 26, color: '" + ACC + "', flex: 1 }}>仍可用</Text>\n"
            "          <Text style={{ fontSize: 26, color: '" + TXT + "', opacity: 0.7, flex: 1 }}>多受限</Text>\n"
            "        </Box>\n"
            "      </Box>\n"
            "      " + hl_card(width=1000, height=130, pad='0 30px') + "\n"
            "          <Text style={{ fontSize: 32, fontWeight: 'bold', color: '" + TXT + "', textAlign: 'center', lineHeight: 1.5 }}>别人填表单，我们说一句话；<br /><span style={{ color: '" + ACC + "' }}>提醒你停下来，而不是催你干活。</span></Text>\n"
            "        </Box>\n"
            "    </Box>")

# ---------------- P29 结尾 ----------------
P29 = """<Slide style={{ width: 1280, height: 720, background: '%s', padding: '64px 80px', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 22 }}>
    <svg width={200} height={200} viewBox='0 0 200 200'>
      <circle cx='100' cy='100' r='92' fill='none' stroke='%s' strokeWidth='1' opacity='0.22' />
      <circle cx='100' cy='100' r='64' fill='none' stroke='%s' strokeWidth='1.5' opacity='0.4' />
      <path d='M100 100 L176 56' fill='none' stroke='%s' strokeWidth='2' opacity='0.7' />
      <path d='M148 64 A92 92 0 0 1 176 100' fill='none' stroke='%s' strokeWidth='2.5' opacity='0.9' />
      <circle cx='100' cy='100' r='4' fill='%s' />
      <circle cx='176' cy='56' r='3.5' fill='%s' />
    </svg>
    <Text style={{ fontSize: 56, fontWeight: 'bold', color: '%s', lineHeight: 1.2 }}>知时而歇，张弛有度</Text>
    <Text style={{ fontSize: 30, color: '%s', letterSpacing: 2 }}>提醒你停下来，而不是催你干活</Text>
    <Box style={{ height: 3, width: 120, background: 'linear-gradient(135deg, #4FE0B0 0%%, #2E9E8F 100%%)', borderRadius: 2 }} />
    <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 12, marginTop: 6 }}>
      <svg width={26} height={26} viewBox='0 0 22 22'>
        <circle cx='11' cy='11' r='10' fill='rgba(79,224,176,0.12)' stroke='%s' strokeWidth='1.5' />
        <path d='M6 11 L9.5 14.5 L16 8' fill='none' stroke='%s' strokeWidth='2' />
      </svg>
      <Text style={{ fontSize: 28, color: '%s' }}>已经做完 · 双平台可直接下载安装</Text>
    </Box>
    <Text style={{ fontSize: 20, color: '%s', fontFamily: 'Consolas', marginTop: 6 }}>https://smileher.github.io/ZhiXie/</Text>
    <Text style={{ fontSize: 22, color: '%s', opacity: 0.5, letterSpacing: 1, marginTop: 6 }}>创新主赛道 · AIGC 创新组</Text>
</Slide>
""" % (BG, ACC, ACC, ACC, ACC, ACC, ACC, TXT, ACC, ACC, ACC, ACC, ACC, ACC)

# ---------------- 组装 ----------------
pages = [
    ('01.slide', P01),
    ('02.slide', P02),
    ('03.slide', content_slide(3, 0, '一坐就是三四个小时', P03_BODY)),
    ('04.slide', content_slide(4, 0, '现有方案，总差一口气', P04_BODY)),
    ('05.slide', content_slide(5, 0, '知歇是什么', P05_BODY)),
    ('06.slide', content_slide(6, 1, '用户画像 · 学生群体', P06_BODY)),
    ('07.slide', content_slide(7, 1, '用户画像 · 职场人群', P07_BODY)),
    ('08.slide', content_slide(8, 1, '调研数据支撑', P08_BODY)),
    ('09.slide', content_slide(9, 2, '场景一 · 学生与备考', P09_BODY)),
    ('10.slide', content_slide(10, 2, '场景二 · 职场久坐', P10_BODY)),
    ('11.slide', content_slide(11, 2, '场景三 · 跨平台随身', P11_BODY)),
    ('12.slide', content_slide(12, 3, '一个人 + AI 助手怎么配合', P12_BODY)),
    ('13.slide', content_slide(13, 3, '提示词与模型选型', P13_BODY)),
    ('14.slide', content_slide(14, 3, '双通道架构总览', P14_BODY)),
    ('15.slide', content_slide(15, 3, '校验层 · AI 说的先核一遍', P15_BODY)),
    ('16.slide', content_slide(16, 3, '调试实录 + 138 项兜底', P16_BODY)),
    ('17.slide', content_slide(17, 4, '实施计划总览', P17_BODY)),
    ('18.slide', content_slide(18, 4, '阶段一 · 开发内测', P18_BODY)),
    ('19.slide', content_slide(19, 4, '阶段二 · 发布参赛', P19_BODY)),
    ('20.slide', content_slide(20, 4, '质量保障怎么落地', P20_BODY)),
    ('21.slide', content_slide(21, 4, '风险与应对', P21_BODY)),
    ('22.slide', content_slide(22, 5, '已经做成的成果', P22_BODY)),
    ('23.slide', content_slide(23, 5, '可交付物清单', P23_BODY)),
    ('24.slide', content_slide(24, 6, '创新①② · 一句话建提醒 + AI 校验', P24_BODY)),
    ('25.slide', content_slide(25, 6, '创新③④ · 人机协同 + 立场', P25_BODY)),
    ('26.slide', content_slide(26, 6, '创新⑤ · 双通道 + 本机数据', P26_BODY)),
    ('27.slide', content_slide(27, 7, '应用价值 · 三层', P27_BODY)),
    ('28.slide', content_slide(28, 7, '应用价值 · 对比与收束', P28_BODY)),
    ('29.slide', P29),
]

os.makedirs(OUT, exist_ok=True)
for name, jsx in pages:
    with open(os.path.join(OUT, name), 'w', encoding='utf-8') as f:
        f.write(jsx)
    print('OK', name)
print('total', len(pages))
